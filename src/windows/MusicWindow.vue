<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { LogicalSize, PhysicalPosition } from '@tauri-apps/api/dpi'
import { emit as emitEvent, listen } from '@tauri-apps/api/event'
import { currentMonitor, getCurrentWindow, primaryMonitor, type Monitor } from '@tauri-apps/api/window'
import { open } from '@tauri-apps/plugin-dialog'
import MusicVisualizerCanvas from '../components/MusicVisualizerCanvas.vue'
import MusicWebglStarfield from '../components/MusicWebglStarfield.vue'
import {
  useMusicAudioAnalyzer,
  type MusicEnergyFrame,
} from '../composables/useMusicAudioAnalyzer'
import { useMusicBeatMapAnalyzer } from '../composables/useMusicBeatMapAnalyzer'
import { useMusicLyrics } from '../composables/useMusicLyrics'
import { useWindowOpenAnimation } from '../composables/useWindowOpenAnimation'
import { DEFAULT_MUSIC_STAGE_TUNING } from '../types/app'
import type {
  DrawerTheme,
  MusicImmersiveTheme,
  MusicImmersiveThemePreference,
  MusicLineStyle,
  MusicRippleStyle,
  MusicStageTuning,
  MusicSpectrumStyle,
  MusicVisualStagePreset,
  PetDrawerConfig,
} from '../types/app'

type RepeatMode = 'none' | 'one' | 'all'
type MusicLibraryView = 'all' | 'favorites' | 'recent' | 'queue'
type MusicPanelView = 'library' | 'netease' | 'kugou'
type MusicRecommendationSource = 'smart' | 'tags' | 'favorites' | 'recent'
type MusicVisualMode = 'rhythm' | 'dance' | 'focus' | 'sleep'
type KugouContentSource = 'personal' | 'recommended' | ''
type MusicStageTuningMap = Record<MusicVisualStagePreset, MusicStageTuning>
type MusicLyricStagePreset = 'clear' | 'projection' | 'float'
type OnlinePlaybackQualityPlatform = 'general' | 'netease' | 'kugou'
type OnlinePlaybackQuality =
  | 'highest'
  | 'jymaster'
  | 'sky'
  | 'jyeffect'
  | 'hires'
  | 'lossless'
  | 'exhigh'
  | 'high'
  | 'standard'
  | 'viper_clear'
  | 'viper_hifi'
  | 'viper_tape'
  | 'viper_atmos'
  | 'multitrack'
  | 'super'
  | 'flac'
  | '320'
  | '128'
interface MusicStagePresetOption {
  value: MusicVisualStagePreset
  label: string
  kicker: string
  description: string
  mode: MusicVisualMode
  spectrumStyle: MusicSpectrumStyle
  lineStyle: MusicLineStyle
  rippleStyle: MusicRippleStyle
  swatches: string[]
  metrics: string[]
}
type MusicStageTuningKey = Exclude<keyof MusicStageTuning, 'centerPulse'>
interface MusicStageTuningOption {
  key: MusicStageTuningKey
  label: string
  min: number
  max: number
  step: number
  ariaLabel: string
}
interface OnlinePlaybackQualityOption {
  value: OnlinePlaybackQuality
  label: string
  description: string
  disabled?: boolean
  availabilityStatus?: KugouQualityAvailabilityStatus
  availabilityReason?: string
  availabilityDetail?: string
}

type KugouQualityAvailabilityStatus = 'available' | 'unavailable' | 'unknown'

interface KugouQualityAvailabilityItem {
  quality: OnlinePlaybackQuality
  label: string
  status: KugouQualityAvailabilityStatus
  reason?: string | null
  detail?: string | null
}
interface MusicLyricStageDefaults {
  depth: number
  tilt: number
  glow: number
  fontScale: number
  vertical: number
  width: number
  sideOpacity: number
  progressGlow: number
  particles: boolean
  cameraLock: boolean
}
interface WebglLyricStageState {
  active: boolean
  textMode: 'lyric' | 'loading' | 'empty' | 'error' | 'placeholder'
  statusText: string
  current: string
  currentKey: string
  previous: string
  previousKey: string
  next: string
  nextKey: string
  progress: number
  status: string
  synced: boolean
  interlude: boolean
  fontScale: number
  tilt: number
  glow: number
  verticalOffsetPx: number
  distanceOffsetPx: number
  distanceScale: number
  sideOpacity: number
  currentLines: number
  sideLines: number
}
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
  source?: 'local' | 'netease' | 'kugou'
  coverImgUrl?: string | null
  neteaseSongId?: number
  kugouSongHash?: string
  category: string
  tags: string[]
  url: string
  duration: number | null
  playbackLevel?: string | null
  playbackBitrate?: number | null
  playbackFileType?: string | null
  playbackSize?: number | null
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
  coverImgUrl?: string | null
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
  coverImgUrl: string
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

interface PlatformMembershipInfo {
  active: boolean
  statusLabel: string
  typeLabel?: string | null
  levelLabel?: string | null
  expireAt?: string | null
}

interface NeteaseLoginProfile {
  userId: number
  nickname: string
  avatarUrl?: string | null
  membership?: PlatformMembershipInfo | null
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

interface KugouLoginProfile {
  userId: string
  nickname: string
  avatarUrl?: string | null
  membership?: PlatformMembershipInfo | null
}

interface KugouLoginStatus {
  loggedIn: boolean
  profile: KugouLoginProfile | null
  savedAt: string | null
  checkedAt: string
  message: string
}

interface KugouQrLogin {
  key: string
  qrUrl: string
  qrImage: string
  expiresAt: string
}

interface KugouQrCheckResult {
  code: number
  status: string
  message: string
  loggedIn: boolean
  profile: KugouLoginProfile | null
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

interface NeteaseSearchResult {
  keyword: string
  tracks: NeteasePlaylistTrack[]
  total: number
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

interface KugouSearchTrack {
  id: string
  hash: string
  hashCandidates?: string[]
  name: string
  artists: string[]
  album?: string | null
  durationMs?: number | null
  coverImgUrl?: string | null
  albumId?: string | null
  albumAudioId?: number | null
  audioId?: number | null
  privilege?: number | null
  payType?: number | null
}

interface KugouSearchResult {
  keyword: string
  tracks: KugouSearchTrack[]
  total: number
  message: string
}

interface KugouPlaylistSummary {
  id: string
  listId: string
  globalCollectionId?: string | null
  name: string
  trackCount: number
  coverImgUrl?: string | null
  creatorNickname?: string | null
  subscribed: boolean
  updateTime?: number | null
}

interface KugouPlaylistDetail {
  playlist: KugouPlaylistSummary
  tracks: KugouSearchTrack[]
  totalTrackCount: number
  truncated: boolean
  message: string
}

interface KugouRecommendedPlaylists {
  playlists: KugouPlaylistSummary[]
  total: number
  page: number
  truncated: boolean
  message: string
}

interface KugouLyricsResult {
  songId: string
  content: string
  lrcContent?: string | null
  source: string
  warnings: string[]
}

interface KugouPlaybackUrl {
  hash: string
  url: string
  qualityLevel?: string | null
  qualityLabel?: string | null
  bitrate?: number | null
  durationMs?: number | null
  fileType?: string | null
  size?: number | null
  message: string
  proxyDiagnostic?: string | null
  proxyLikelyPreview: boolean
}

interface KugouPlaybackProxyStatus {
  ok: boolean
  message: string
  probeMessage?: string | null
  likelyPreview: boolean
  lastError?: string | null
  lastRange?: string | null
  refreshCount: number
}

interface KugouQualityAvailability {
  hash: string
  qualities: KugouQualityAvailabilityItem[]
  message: string
  diagnostic?: string | null
}

type ImmersivePlaylistSource = 'local' | 'netease' | 'kugou'
type ImmersiveSearchPlatform = 'netease' | 'kugou'
type ImmersiveSearchPlatformOptionValue = 'all' | ImmersiveSearchPlatform
type MusicPlaybackContext =
  | { source: 'local'; trackIds: string[] }
  | { source: 'netease'; tracks: NeteasePlaylistTrack[] }
  | { source: 'kugou'; tracks: KugouSearchTrack[] }

type ImmersiveSearchResultItem =
  | { key: string; platform: 'netease'; track: NeteasePlaylistTrack; sourceIndex: number }
  | { key: string; platform: 'kugou'; track: KugouSearchTrack; sourceIndex: number }

interface OnlinePlaybackCacheEntry<T> {
  playback: T
  expiresAt: number
}

type OnlinePlaybackPrefetchCandidate =
  | { source: 'netease'; track: NeteasePlaylistTrack }
  | { source: 'kugou'; track: KugouSearchTrack }

interface OnlineUnavailableTrack {
  reason: string
  failedAt: number
  retryAfter: number
}

interface PlaybackFailureDisplay {
  title: string
  summary: string
  compact: string
  detail: string
  hints: string[]
}

interface OnlinePlaybackOptions {
  autoSkip?: boolean
}

interface TrackIdentity {
  title: string
  artist: string
}

interface ImmersiveFreeCameraView {
  active: boolean
  locked: boolean
  x: number
  y: number
  z: number
  yaw: number
  pitch: number
  roll: number
  fov: number
}

type MiniEdgeDockSide = 'left' | 'right' | 'top' | 'bottom'

interface MiniEdgeDockState {
  side: MiniEdgeDockSide
  hiddenX: number
  hiddenY: number
  expandedX: number
  expandedY: number
}

interface MiniEdgeWindowMoveOptions {
  animated?: boolean
  durationMs?: number
}

interface ScreenWorkArea {
  left: number
  top: number
  right: number
  bottom: number
}

const ALL_CATEGORY = '全部'
const ALL_TAG = '全部标签'
const DEFAULT_CATEGORY = '未分类'
const MAX_PLAY_HISTORY_PER_TRACK = 30
const MAX_TRACK_TAGS = 12
const MAX_TRACK_TAG_LENGTH = 24
const FULL_MUSIC_WINDOW_SIZE = { width: 960, height: 700 }
const MINI_MUSIC_WINDOW_SIZE = { width: 344, height: 154 }
const MINI_EDGE_SNAP_DISTANCE = 96
const MINI_EDGE_VISIBLE_STRIP = 32
const MINI_EDGE_REVEAL_MARGIN = 10
const MINI_EDGE_MOVE_DEBOUNCE_MS = 360
const MINI_EDGE_REHIDE_DELAY_MS = 520
const MINI_EDGE_AUTO_HIDE_DELAY_MS = 1000
const MINI_EDGE_POSITION_SUPPRESS_MS = 500
const MINI_EDGE_DRAG_POLL_INTERVAL_MS = 160
const MINI_EDGE_DRAG_POLL_IDLE_TICKS = 3
const MINI_EDGE_DRAG_POLL_MAX_MS = 4200
const MINI_EDGE_ANIMATION_STEP_MS = 16
const MINI_EDGE_SNAP_ANIMATION_MS = 180
const MINI_EDGE_HIDE_ANIMATION_MS = 240
const MINI_EDGE_REVEAL_ANIMATION_MS = 180
const TRACKS_STORAGE_KEY = 'pet-drawer-music-tracks'
const SETTINGS_STORAGE_KEY = 'pet-drawer-music-settings'
const PLAYLISTS_STORAGE_KEY = 'pet-drawer-music-playlists'
const ONLINE_PLAYBACK_CACHE_TTL_MS = 8 * 60 * 1000
const ONLINE_UNAVAILABLE_RETRY_AFTER_MS = 10 * 60 * 1000
const ONLINE_PLAYBACK_PREFETCH_DELAY_MS = 2600
const KUGOU_QUALITY_AVAILABILITY_CACHE_TTL_MS = 5 * 60 * 1000
const DEFAULT_ONLINE_PLAYBACK_QUALITY: OnlinePlaybackQuality = 'highest'
const DEFAULT_NETEASE_ONLINE_PLAYBACK_QUALITY: OnlinePlaybackQuality = 'highest'
const DEFAULT_KUGOU_ONLINE_PLAYBACK_QUALITY: OnlinePlaybackQuality = 'viper_clear'
const GENERAL_ONLINE_PLAYBACK_QUALITY_OPTIONS: OnlinePlaybackQualityOption[] = [
  { value: 'highest', label: '最高可用', description: '按当前平台最高规格优先' },
  { value: 'lossless', label: '无损优先', description: '平台无损或 FLAC 优先' },
  { value: 'high', label: '高音质', description: '320kbps 优先' },
  { value: 'standard', label: '标准', description: '128kbps' },
]
const NETEASE_ONLINE_PLAYBACK_QUALITY_OPTIONS: OnlinePlaybackQualityOption[] = [
  { value: 'highest', label: '超清母带', description: 'SVIP / 最高规格可用时优先' },
  { value: 'sky', label: '沉浸环绕', description: '空间音频优先' },
  { value: 'jyeffect', label: '高清环绕', description: '环绕音效优先' },
  { value: 'hires', label: 'Hi-Res', description: '高解析优先' },
  { value: 'lossless', label: '无损 SQ', description: 'FLAC 优先' },
  { value: 'exhigh', label: '极高 HQ', description: '320kbps' },
  { value: 'standard', label: '标准', description: '128kbps' },
]
const KUGOU_ONLINE_PLAYBACK_QUALITY_OPTIONS: OnlinePlaybackQualityOption[] = [
  { value: 'viper_clear', label: '蝰蛇母带音质', description: '超级 VIP 独享 / 传统 CD 音质 4 倍' },
  { value: 'super', label: '蝰蛇超清音质', description: '超级 VIP 独享 / 比黑胶唱片更清晰' },
  { value: 'viper_hifi', label: '蝰蛇HIFI音质', description: '超级 VIP 独享 / 低频震撼高频通透' },
  { value: 'viper_atmos', label: '蝰蛇全景声2.0', description: '超级 VIP 独享 / 沉浸式音乐环境感' },
  { value: 'hires', label: 'Hi-Res音质', description: '高解析音频优先' },
  { value: 'flac', label: '无损音质', description: 'FLAC 优先' },
  { value: 'high', label: '高品音质', description: '320kbps' },
  { value: 'standard', label: '标准音质', description: '128kbps' },
]
const ALL_ONLINE_PLAYBACK_QUALITY_OPTIONS = [
  ...GENERAL_ONLINE_PLAYBACK_QUALITY_OPTIONS,
  ...NETEASE_ONLINE_PLAYBACK_QUALITY_OPTIONS,
  ...KUGOU_ONLINE_PLAYBACK_QUALITY_OPTIONS,
]
const IMMERSIVE_CONTENT_PREP_DELAY_MS = 1400
const PLATFORM_SEARCH_PAGE_SIZE = 50
const IMMERSIVE_SEARCH_PAGE_SIZE = 18
const MAX_PLATFORM_SEARCH_PAGE = 20
const PLATFORM_PLAYLIST_PAGE_SIZE = 300
const MAX_PLATFORM_PLAYLIST_PAGE = 200
const PLAYBACK_CONTEXT_PREFETCH_THRESHOLD = 2
const IMMERSIVE_PLAYLIST_CONTEXT_RADIUS = 50
const IMMERSIVE_PLAYLIST_FALLBACK_LIMIT = IMMERSIVE_PLAYLIST_CONTEXT_RADIUS * 2 + 1
const IMMERSIVE_LYRIC_MAIN_MAX_LINES = 4
const IMMERSIVE_LYRIC_SIDE_MAX_LINES = 2
const IMMERSIVE_STAGE_MAX_YAW = 22
const IMMERSIVE_STAGE_MAX_PITCH = 12
const IMMERSIVE_STAGE_DRAG_YAW_FACTOR = 0.085
const IMMERSIVE_STAGE_DRAG_PITCH_FACTOR = 0.062
const LYRIC_STAGE_FONT_SCALE_MIN = 0.66
const LYRIC_STAGE_FONT_SCALE_MAX = 1.38
const LYRIC_STAGE_VERTICAL_OFFSET_SPAN_PX = 520
const LYRIC_STAGE_DISTANCE_DEFAULT = 0.5
const LYRIC_STAGE_DISTANCE_OFFSET_SPAN_PX = 360
const LYRIC_STAGE_DISTANCE_SCALE_MIN = 0.96
const LYRIC_STAGE_DISTANCE_SCALE_MAX = 1.04
const DJ_FLOAT_LYRIC_Y_OFFSET = 58
const DJ_FLOAT_LYRIC_Z_OFFSET = 42
const DJ_FLOAT_LYRIC_SCALE = 0.94
const LYRIC_MUSIC_ENVELOPE_ZERO = {
  pulse: 0,
  breath: 0,
  phrase: 0,
  air: 0,
  drift: 0,
  focus: 0,
}
const LYRIC_STAGE_PRESET_DEFAULTS: Record<MusicLyricStagePreset, MusicLyricStageDefaults> = {
  clear: {
    depth: 0.64,
    tilt: 0.08,
    glow: 0.28,
    fontScale: 0.56,
    vertical: 0.44,
    width: 0.52,
    sideOpacity: 0.43,
    progressGlow: 0.54,
    particles: false,
    cameraLock: false,
  },
  projection: {
    depth: 0.42,
    tilt: 0.48,
    glow: 0.48,
    fontScale: 0.5,
    vertical: 0.5,
    width: 0.5,
    sideOpacity: 0.87,
    progressGlow: 0.8,
    particles: false,
    cameraLock: false,
  },
  float: {
    depth: 0.76,
    tilt: 0.58,
    glow: 0.56,
    fontScale: 0.48,
    vertical: 0.38,
    width: 0.55,
    sideOpacity: 0.72,
    progressGlow: 0.72,
    particles: true,
    cameraLock: true,
  },
}
const IMMERSIVE_FREE_CAMERA_DEFAULT = {
  x: 0,
  y: 0.7,
  z: 8.2,
  yaw: 0,
  pitch: -0.08,
  roll: 0,
  fov: 46,
}
const IMMERSIVE_FREE_CAMERA_MOUSE_FACTOR = 0.00125
const IMMERSIVE_FREE_CAMERA_BASE_SPEED = 2.35
const IMMERSIVE_FREE_CAMERA_FAST_SPEED = 6.2
const IMMERSIVE_FREE_CAMERA_RESET_MS = 620
const IMMERSIVE_FREE_CAMERA_CONTROL_CODES = new Set([
  'KeyW',
  'KeyA',
  'KeyS',
  'KeyD',
  'KeyQ',
  'KeyE',
  'Space',
  'ShiftLeft',
  'ShiftRight',
  'ControlLeft',
  'ControlRight',
])
const ONLINE_STALL_RECOVERY_DELAY_MS = 7000
const ONLINE_STALL_RECOVERY_RETRY_DELAY_MS = 1800
const ONLINE_STALL_RECOVERY_VERIFY_MS = 9000
const ONLINE_STALL_RECOVERY_PROGRESS_SECONDS = 0.8
const MAX_ONLINE_STALL_RECOVERY_ATTEMPTS = 3
const NETEASE_TRACK_ID_PREFIX = 'netease'
const KUGOU_TRACK_ID_PREFIX = 'kugou'
const IMMERSIVE_SEARCH_PLATFORMS: ImmersiveSearchPlatform[] = ['netease', 'kugou']
const immersiveSearchPlatformOptions: Array<{
  value: ImmersiveSearchPlatformOptionValue
  label: string
}> = [
  { value: 'all', label: '全部' },
  { value: 'netease', label: '网易云' },
  { value: 'kugou', label: '酷狗' },
]
const panelViewOptions: Array<{ value: MusicPanelView; label: string }> = [
  { value: 'library', label: '本地音乐' },
  { value: 'netease', label: '网易云' },
  { value: 'kugou', label: '酷狗' },
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
const stagePresetOptions: MusicStagePresetOption[] = [
  {
    value: 'galaxy',
    label: '星河漫游',
    kicker: 'SPACE',
    description: '高密星尘、星云旋臂和流星轨迹集中在这里，突出宇宙纵深。',
    mode: 'focus',
    spectrumStyle: 'particles',
    lineStyle: 'constellation',
    rippleStyle: 'halo',
    swatches: ['#8fe9ff', '#73a7ff', '#fff0b8'],
    metrics: ['星尘高', '星云深'],
  },
  {
    value: 'dj',
    label: '地面 DJ',
    kicker: 'COLUMNS',
    description: '一列列地面 EQ 柱跟随频段起伏，扫描线和轨迹流保持长播律动。',
    mode: 'dance',
    spectrumStyle: 'orbit',
    lineStyle: 'scan',
    rippleStyle: 'heartbeat',
    swatches: ['#00f5d4', '#ff4fd8', '#7df9ff'],
    metrics: ['柱阵起', '扫描快'],
  },
]
const defaultVisualStagePreset: MusicVisualStagePreset = 'dj'
const defaultVisualStagePresetOption =
  stagePresetOptions.find((option) => option.value === defaultVisualStagePreset) ?? stagePresetOptions[0]
const galaxyStageTuningOptions: MusicStageTuningOption[] = [
  { key: 'density', label: '星量', min: 0.35, max: 2, step: 0.01, ariaLabel: '星河漫游粒子数量' },
  { key: 'response', label: '流速', min: 0.25, max: 2.4, step: 0.01, ariaLabel: '星河漫游星尘流速' },
  { key: 'height', label: '纵深', min: 0.45, max: 2.2, step: 0.01, ariaLabel: '星河漫游空间纵深' },
  { key: 'layerHeight', label: '层高', min: 0.25, max: 2.8, step: 0.01, ariaLabel: '星河漫游上下分布层高' },
  { key: 'wave', label: '星云', min: 0.15, max: 2.6, step: 0.01, ariaLabel: '星河漫游星云强度' },
  { key: 'trigger', label: '闪耀', min: 0.1, max: 2.8, step: 0.01, ariaLabel: '星河漫游星点闪耀强度' },
  { key: 'camera', label: '视距', min: 0.55, max: 1.75, step: 0.01, ariaLabel: '星河漫游默认视距' },
]
const djStageTuningOptions: MusicStageTuningOption[] = [
  { key: 'height', label: '高度', min: 0.25, max: 2.4, step: 0.01, ariaLabel: '地面 DJ 柱体高度' },
  { key: 'response', label: '响应', min: 0.2, max: 2.4, step: 0.01, ariaLabel: '地面 DJ 音乐响应速度' },
  { key: 'density', label: '密度', min: 0.35, max: 1.7, step: 0.01, ariaLabel: '地面 DJ 柱阵密度' },
  { key: 'wave', label: '波峰', min: 0.2, max: 2.4, step: 0.01, ariaLabel: '地面 DJ 触发点扩散波峰强度' },
  { key: 'trigger', label: '触发', min: 0.25, max: 2.4, step: 0.01, ariaLabel: '地面 DJ 音乐触发点敏感度' },
  { key: 'camera', label: '视距', min: 0.55, max: 1.65, step: 0.01, ariaLabel: '地面 DJ 默认视距' },
]
const spectrumStyleOptions: Array<{ value: MusicSpectrumStyle; label: string; description: string }> = [
  { value: 'bars', label: '竖条', description: '经典实时频谱，稳定清晰。' },
  { value: 'mirror', label: '棱柱', description: '上下镜像频谱，突出低频冲击。' },
  { value: 'orbit', label: '星盘', description: '环形刻度围绕中心旋转响应。' },
  { value: 'particles', label: '星辰', description: '粒子星光带柔光光晕，并随节拍、低频和音量明显闪烁。' },
  { value: 'ribbon', label: '丝带', description: '连续流光曲线表现中高频流动。' },
  { value: 'none', label: '隐藏', description: '不显示实时频谱层。' },
]
const lineStyleOptions: Array<{ value: MusicLineStyle; label: string; description: string }> = [
  { value: 'wave', label: '柔波', description: '连续波线，适合歌词和低干扰播放。' },
  { value: 'beam', label: '光束', description: '多层横向光带，带舞台灯效果。' },
  { value: 'scan', label: '扫描', description: '细扫描线随节奏横向移动。' },
  { value: 'constellation', label: '星轨', description: '星点连线随节拍、低频和音量明显扩张。' },
  { value: 'none', label: '隐藏', description: '不显示线条层。' },
]
const rippleStyleOptions: Array<{ value: MusicRippleStyle; label: string; description: string }> = [
  { value: 'rings', label: '圆环', description: '经典扩散波纹。' },
  { value: 'water', label: '水波', description: '横向水面涟漪，柔和低干扰。' },
  { value: 'heartbeat', label: '心跳', description: '低频短促冲击波。' },
  { value: 'halo', label: '光晕', description: '中心柔光随能量呼吸。' },
  { value: 'none', label: '隐藏', description: '不显示波纹层。' },
]
const immersiveThemeOptions: Array<{
  value: MusicImmersiveThemePreference
  label: string
  description: string
  swatches: string[]
}> = [
  {
    value: 'follow',
    label: '跟随',
    description: '沿用桌宠主题色。',
    swatches: ['#ffcf7a', '#7fb6a6', '#10192a'],
  },
  {
    value: 'light',
    label: '默认',
    description: '清爽深色舞台。',
    swatches: ['#ffcf7a', '#2aa7d9', '#10192a'],
  },
  {
    value: 'animal-island',
    label: '动物岛',
    description: '暖纸感岛屿光。',
    swatches: ['#ffe7ad', '#7fb6a6', '#69522d'],
  },
  {
    value: 'cinema',
    label: '暗场',
    description: '胶片金色信号。',
    swatches: ['#f4d28a', '#ff5367', '#090a0d'],
  },
  {
    value: 'galaxy',
    label: '星河',
    description: '青蓝粒子空间。',
    swatches: ['#9cffdf', '#73a7ff', '#06101d'],
  },
  {
    value: 'neon',
    label: '霓虹',
    description: '高对比节拍脉冲。',
    swatches: ['#00f5d4', '#ff4fd8', '#150b3e'],
  },
  {
    value: 'sunset',
    label: '暖场',
    description: '香槟橙红舞台。',
    swatches: ['#f4d28a', '#ff8a5c', '#2b1115'],
  },
  {
    value: 'midnight',
    label: '深夜',
    description: '低亮度蓝灰光。',
    swatches: ['#9fb7d9', '#b8cdfa', '#050812'],
  },
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
const miniPlayerElement = ref<HTMLElement | null>(null)
const neteasePlaylistDetailSection = ref<HTMLElement | null>(null)
const kugouPlaylistDetailSection = ref<HTMLElement | null>(null)
const musicWindow = getCurrentWindow()
const { windowOpenAnimationClass } = useWindowOpenAnimation('panel')
const immersiveScene = ref<HTMLElement | null>(null)
const immersiveSearchRoot = ref<HTMLElement | null>(null)
const immersiveSearchInput = ref<HTMLInputElement | null>(null)
const tracks = ref<MusicTrack[]>([])
const playQueue = ref<string[]>([])
const playbackContext = ref<MusicPlaybackContext>({ source: 'local', trackIds: [] })
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
const onlinePlaybackQuality = ref<OnlinePlaybackQuality>(DEFAULT_ONLINE_PLAYBACK_QUALITY)
const neteaseOnlinePlaybackQuality = ref<OnlinePlaybackQuality>(DEFAULT_NETEASE_ONLINE_PLAYBACK_QUALITY)
const kugouOnlinePlaybackQuality = ref<OnlinePlaybackQuality>(DEFAULT_KUGOU_ONLINE_PLAYBACK_QUALITY)
const onlinePlaybackQualitySwitching = ref(false)
const immersiveQualityMenuOpen = ref(false)
const playerErrorDetailOpen = ref(false)
const drawerTheme = ref<DrawerTheme>('light')
const musicImmersiveThemePreference = ref<MusicImmersiveThemePreference>('follow')
const immersiveThemeSaving = ref(false)
const immersiveThemeError = ref('')
const settingsVisible = ref(false)
const miniPlayerMode = ref(false)
const miniEdgeDockSide = ref<MiniEdgeDockSide | null>(null)
const miniEdgeDockExpanded = ref(false)
const immersiveMode = ref(false)
const immersiveStageOnlyMode = ref(false)
const listFocusMode = ref(false)
const immersivePlaylistVisible = ref(true)
const immersiveRhythmPanelVisible = ref(true)
const immersivePlaylistSource = ref<ImmersivePlaylistSource>('local')
const immersiveSearchQuery = ref('')
const immersiveSearchFocused = ref(false)
const immersiveSearchLoading = ref(false)
const immersiveSearchExecuted = ref(false)
const immersiveSearchError = ref('')
const immersiveSearchNotice = ref('')
const immersiveSearchPlatforms = ref<ImmersiveSearchPlatform[]>([...IMMERSIVE_SEARCH_PLATFORMS])
const immersiveNeteaseSearchResult = ref<NeteaseSearchResult | null>(null)
const immersiveKugouSearchResult = ref<KugouSearchResult | null>(null)
const visualStagePreset = ref<MusicVisualStagePreset>(defaultVisualStagePresetOption.value)
const visualMode = ref<MusicVisualMode>(defaultVisualStagePresetOption.mode)
const visualSpectrumStyle = ref<MusicSpectrumStyle>(defaultVisualStagePresetOption.spectrumStyle)
const visualLineStyle = ref<MusicLineStyle>(defaultVisualStagePresetOption.lineStyle)
const visualRippleStyle = ref<MusicRippleStyle>(defaultVisualStagePresetOption.rippleStyle)
const visualIntensity = ref(0.72)
const visualReducedMotion = ref(false)
const visualStageTunings = ref<MusicStageTuningMap>(createDefaultMusicStageTunings())
const stageTuningOptions = computed(() => stageTuningOptionsForPreset(visualStagePreset.value))
const visualStageTuning = computed(() => visualStageTunings.value[visualStagePreset.value])
const lyricStageTilt = ref(LYRIC_STAGE_PRESET_DEFAULTS.projection.tilt)
const lyricStageGlow = ref(LYRIC_STAGE_PRESET_DEFAULTS.projection.glow)
const lyricStageFontScale = ref(LYRIC_STAGE_PRESET_DEFAULTS.projection.fontScale)
const lyricStageVertical = ref(LYRIC_STAGE_PRESET_DEFAULTS.projection.vertical)
const lyricStageDistance = ref(LYRIC_STAGE_DISTANCE_DEFAULT)
const lyricStageSideOpacity = ref(LYRIC_STAGE_PRESET_DEFAULTS.projection.sideOpacity)
const webglStarfieldUnavailable = ref(false)
const immersiveStageDragging = ref(false)
const immersiveStageYaw = ref(0)
const immersiveStagePitch = ref(0)
const immersiveStageVelocityYaw = ref(0)
const immersiveStageVelocityPitch = ref(0)
const immersiveFreeCameraActive = ref(false)
const immersiveFreeCameraLocked = ref(false)
const immersiveFreeCameraResetting = ref(false)
const immersiveFreeCameraX = ref(IMMERSIVE_FREE_CAMERA_DEFAULT.x)
const immersiveFreeCameraY = ref(IMMERSIVE_FREE_CAMERA_DEFAULT.y)
const immersiveFreeCameraZ = ref(IMMERSIVE_FREE_CAMERA_DEFAULT.z)
const immersiveFreeCameraYaw = ref(IMMERSIVE_FREE_CAMERA_DEFAULT.yaw)
const immersiveFreeCameraPitch = ref(IMMERSIVE_FREE_CAMERA_DEFAULT.pitch)
const immersiveFreeCameraRoll = ref(IMMERSIVE_FREE_CAMERA_DEFAULT.roll)
const immersiveFreeCameraFov = ref(IMMERSIVE_FREE_CAMERA_DEFAULT.fov)
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
const neteaseSearchQuery = ref('')
const neteaseSearchResult = ref<NeteaseSearchResult | null>(null)
const neteaseSearchLoading = ref(false)
const neteaseSearchError = ref('')
const neteaseSearchNotice = ref('')
const neteaseSearchPage = ref(0)
const neteasePlaylists = ref<NeteasePlaylistSummary[]>([])
const neteaseSelectedPlaylistId = ref<number | null>(null)
const neteasePlaylistDetail = ref<NeteasePlaylistDetail | null>(null)
const neteasePlaylistsLoading = ref(false)
const neteasePlaylistDetailLoading = ref(false)
const neteasePlaylistDetailPage = ref(0)
const neteasePlaylistError = ref('')
const neteaseTrackActionId = ref<number | null>(null)
const neteaseCurrentTrack = ref<MusicTrack | null>(null)
const neteaseLyricsTrack = ref<NeteasePlaylistTrack | null>(null)
const neteaseLyricsResult = ref<NeteaseLyricsResult | null>(null)
const neteaseLyricsLoading = ref(false)
const neteaseLyricsError = ref('')
const kugouLoginStatus = ref<KugouLoginStatus | null>(null)
const kugouQrLogin = ref<KugouQrLogin | null>(null)
const kugouLoginBusy = ref(false)
const kugouQrChecking = ref(false)
const kugouQrStatus = ref<'idle' | 'waiting' | 'scanned' | 'expired' | 'authorized' | 'error'>(
  'idle',
)
const kugouLoginNotice = ref('')
const kugouLoginError = ref('')
const kugouPlaylists = ref<KugouPlaylistSummary[]>([])
const kugouSelectedPlaylistId = ref('')
const kugouSelectedRecommendedPlaylistId = ref('')
const kugouSelectedContentSource = ref<KugouContentSource>('')
const kugouPlaylistDetail = ref<KugouPlaylistDetail | null>(null)
const kugouPlaylistsLoading = ref(false)
const kugouPlaylistDetailLoading = ref(false)
const kugouPlaylistDetailPage = ref(0)
const kugouPlaylistError = ref('')
const kugouRecommendedPlaylists = ref<KugouPlaylistSummary[]>([])
const kugouRecommendedPlaylistsLoading = ref(false)
const kugouRecommendedPlaylistPage = ref(0)
const kugouRecommendedPlaylistTotal = ref(0)
const kugouRecommendedPlaylistHasMore = ref(false)
const kugouRecommendedPlaylistError = ref('')
const kugouRecommendedPlaylistNotice = ref('')
const kugouDailyRecommendation = ref<KugouSearchResult | null>(null)
const kugouDailyRecommendationLoading = ref(false)
const kugouDailyRecommendationError = ref('')
const kugouDailyRecommendationNotice = ref('')
const kugouSearchQuery = ref('')
const kugouSearchResult = ref<KugouSearchResult | null>(null)
const kugouSearchLoading = ref(false)
const kugouSearchError = ref('')
const kugouSearchNotice = ref('')
const kugouSearchPage = ref(0)
const kugouTrackActionHash = ref('')
const kugouLyricsTrack = ref<KugouSearchTrack | null>(null)
const kugouLyricsResult = ref<KugouLyricsResult | null>(null)
const kugouLyricsLoading = ref(false)
const kugouLyricsError = ref('')
const neteasePlaybackCache = new Map<string, OnlinePlaybackCacheEntry<NeteasePlaybackUrl>>()
const kugouPlaybackCache = new Map<string, OnlinePlaybackCacheEntry<KugouPlaybackUrl>>()
const kugouQualityAvailabilityCache = reactive(new Map<string, OnlinePlaybackCacheEntry<KugouQualityAvailability>>())
const neteasePlaybackInflight = new Map<string, Promise<NeteasePlaybackUrl>>()
const kugouPlaybackInflight = new Map<string, Promise<KugouPlaybackUrl>>()
const kugouQualityAvailabilityInflight = new Map<string, Promise<KugouQualityAvailability>>()
const neteaseUnavailableTracks = reactive(new Map<number, OnlineUnavailableTrack>())
const kugouUnavailableTracks = reactive(new Map<string, OnlineUnavailableTrack>())
const kugouQualityAvailabilityLoading = ref(false)
const kugouQualityAvailabilityError = ref('')
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
const lyricMusicEnvelope = ref({ ...LYRIC_MUSIC_ENVELOPE_ZERO })
const {
  lyricsStatus,
  lyricsError,
  loadLyricsForTrack,
  resetLyrics,
  lyricsAt,
} = useMusicLyrics()

const currentTrack = computed(() => neteaseCurrentTrack.value ?? tracks.value[currentIndex.value] ?? null)
const currentKugouQualityTrack = computed(() => resolveCurrentKugouQualityTrack())
const currentKugouQualityAvailability = computed(() =>
  currentKugouQualityTrack.value
    ? getCachedKugouQualityAvailability(currentKugouQualityTrack.value)
    : null,
)
const onlinePlaybackQualityPlatform = computed<OnlinePlaybackQualityPlatform>(() =>
  currentOnlinePlaybackQualityPlatform(),
)
const onlinePlaybackQualityOptions = computed(() =>
  onlinePlaybackQualityViewOptionsForPlatform(onlinePlaybackQualityPlatform.value),
)
const activeOnlinePlaybackQuality = computed(() =>
  onlinePlaybackQualityForPlatform(onlinePlaybackQualityPlatform.value),
)
const onlinePlaybackQualityLabel = computed(() =>
  onlinePlaybackQualityOptionLabel(
    activeOnlinePlaybackQuality.value,
    onlinePlaybackQualityPlatform.value,
  ),
)
const onlinePlaybackQualityPlatformLabel = computed(() =>
  onlinePlaybackQualityPlatformText(onlinePlaybackQualityPlatform.value),
)
const onlinePlaybackQualityStatusHint = computed(() => {
  if (onlinePlaybackQualitySwitching.value) {
    return '正在切换当前在线歌曲音质...'
  }
  if (onlinePlaybackQualityPlatform.value === 'kugou' && kugouQualityAvailabilityLoading.value) {
    return '正在预检当前酷狗歌曲可用音质...'
  }
  if (onlinePlaybackQualityPlatform.value === 'kugou' && kugouQualityAvailabilityError.value) {
    return `酷狗音质预检失败：${kugouQualityAvailabilityError.value}`
  }
  return `${onlinePlaybackQualityPlatformLabel.value}：${onlinePlaybackQualityLabel.value} · 播放中切换会从当前进度继续`
})
const editingTrack = computed(() =>
  editingTrackId.value ? trackById(editingTrackId.value) : null,
)
const hasTracks = computed(() => tracks.value.length > 0 || Boolean(neteaseCurrentTrack.value))
const playerErrorDisplay = computed(() =>
  playerError.value ? formatPlaybackFailureDisplay(playerError.value) : null,
)
const hasQueue = computed(() => playQueue.value.length > 0)
const resolvedImmersiveTheme = computed<MusicImmersiveTheme>(() =>
  musicImmersiveThemePreference.value === 'follow'
    ? drawerTheme.value
    : musicImmersiveThemePreference.value,
)
const themeClass = computed(() => `theme-${immersiveMode.value ? resolvedImmersiveTheme.value : drawerTheme.value}`)
const miniEdgeDockClass = computed(() => {
  if (!miniEdgeDockSide.value) {
    return []
  }

  return [
    'music-window-mini-edge-docked',
    `music-window-mini-edge-${miniEdgeDockSide.value}`,
    miniEdgeDockExpanded.value ? 'music-window-mini-edge-expanded' : 'music-window-mini-edge-hidden',
  ]
})
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
  if (activePanelView.value === 'netease') {
    return '网易云音乐'
  }

  if (activePanelView.value === 'kugou') {
    return '酷狗音乐'
  }

  return activeLibraryView.value === 'queue' ? '播放队列' : '播放列表'
})
const playlistCountLabel = computed(() => {
  if (activePanelView.value === 'netease') {
    if (!neteaseLoginStatus.value?.loggedIn) {
      return '等待登录'
    }

    return neteasePlaylists.value.length > 0
      ? `${neteasePlaylists.value.length} 个网易云歌单`
      : '账号已连接'
  }

  if (activePanelView.value === 'kugou') {
    if (kugouSearchLoading.value) {
      return '搜索中'
    }

    return kugouSearchResult.value
      ? `${kugouSearchResult.value.tracks.length} 首酷狗结果`
      : '关键词搜索'
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
const leftLocalPlaylistStatusLabel = computed(() =>
  customPlaylists.value.length > 0
    ? `${customPlaylists.value.length} 个本地歌单`
    : '还没有本地歌单',
)
const leftNeteasePlaylistStatusLabel = computed(() => {
  if (!neteaseLoggedIn.value) {
    return '未登录'
  }

  if (neteasePlaylistsLoading.value) {
    return '读取中'
  }

  return neteasePlaylists.value.length > 0
    ? `${neteasePlaylists.value.length} 个我的歌单`
    : '未读取歌单'
})
const leftKugouPlaylistStatusLabel = computed(() => {
  if (!kugouLoggedIn.value) {
    return '未登录'
  }

  if (kugouPlaylistsLoading.value) {
    return '读取中'
  }

  return kugouPlaylists.value.length > 0
    ? `${kugouPlaylists.value.length} 个我的歌单`
    : '未读取歌单'
})
const leftKugouRecommendedPlaylistStatusLabel = computed(() => {
  if (kugouRecommendedPlaylistsLoading.value) {
    return '读取中'
  }

  if (kugouRecommendedPlaylistError.value) {
    return '读取失败'
  }

  return kugouRecommendedPlaylists.value.length > 0
    ? `${kugouRecommendedPlaylists.value.length} 个推荐`
    : '未读取推荐'
})
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
const neteasePlaylistHasMore = computed(() =>
  Boolean(
    neteasePlaylistDetail.value?.truncated &&
      neteasePlaylistDetailPage.value > 0 &&
      neteasePlaylistDetailPage.value < MAX_PLATFORM_PLAYLIST_PAGE,
  ),
)
const neteasePlaylistLoadedLabel = computed(() => {
  const detail = neteasePlaylistDetail.value
  if (!detail) {
    return '0 首'
  }

  return detail.totalTrackCount > 0
    ? `${detail.tracks.length} / ${detail.totalTrackCount} 首`
    : `${detail.tracks.length} 首`
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
const neteaseSearchTracks = computed(() => neteaseSearchResult.value?.tracks ?? [])
const neteaseSearchHasMore = computed(() => {
  const result = neteaseSearchResult.value
  return Boolean(
    result &&
      result.total > result.tracks.length &&
      neteaseSearchPage.value > 0 &&
      neteaseSearchPage.value < MAX_PLATFORM_SEARCH_PAGE,
  )
})
const neteaseSearchLoadedLabel = computed(() => {
  const result = neteaseSearchResult.value
  if (!result) {
    return `${neteaseSearchTracks.value.length} 首`
  }

  return result.total > 0
    ? `${result.tracks.length} / ${result.total} 首`
    : `${result.tracks.length} 首`
})
const neteaseSearchStatusDetail = computed(() => {
  if (neteaseSearchError.value) {
    return neteaseSearchError.value
  }

  if (neteaseSearchNotice.value) {
    return neteaseSearchNotice.value
  }

  return '输入关键词搜索网易云歌曲；结果只作为当前运行时临时列表。'
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
const kugouLoggedIn = computed(() => Boolean(kugouLoginStatus.value?.loggedIn))
const kugouProfile = computed(() => kugouLoginStatus.value?.profile ?? null)
const kugouLoginStatusLabel = computed(() => {
  if (kugouLoggedIn.value) {
    return '已登录'
  }

  if (kugouQrStatus.value === 'scanned') {
    return '待确认'
  }

  if (kugouQrStatus.value === 'waiting') {
    return '等待扫码'
  }

  if (kugouQrStatus.value === 'expired') {
    return '已过期'
  }

  if (kugouQrStatus.value === 'error') {
    return '连接失败'
  }

  return '未登录'
})
const kugouLoginStatusDetail = computed(() => {
  if (kugouLoginError.value) {
    return kugouLoginError.value
  }

  if (kugouLoginNotice.value) {
    return kugouLoginNotice.value
  }

  return kugouLoginStatus.value?.message ?? '尚未登录酷狗音乐；未登录也可以搜索和尝试临时播放。'
})
const kugouQrExpired = computed(() => {
  if (!kugouQrLogin.value) {
    return false
  }

  const expiresAt = Number(kugouQrLogin.value.expiresAt)
  if (!Number.isFinite(expiresAt)) {
    return false
  }

  return Date.now() >= expiresAt * 1000
})
const kugouSelectedPlaylist = computed(
  () => kugouPlaylists.value.find((playlist) => playlist.listId === kugouSelectedPlaylistId.value) ?? null,
)
const kugouSelectedRecommendedPlaylist = computed(
  () =>
    kugouRecommendedPlaylists.value.find(
      (playlist) => kugouPlaylistKey(playlist) === kugouSelectedRecommendedPlaylistId.value,
    ) ?? null,
)
const kugouActivePlaylist = computed(() => {
  if (kugouSelectedContentSource.value === 'recommended') {
    return kugouSelectedRecommendedPlaylist.value
  }

  return kugouSelectedPlaylist.value
})
const kugouActivePlaylistSourceLabel = computed(() =>
  kugouSelectedContentSource.value === 'recommended' ? '推荐歌单' : '我的歌单',
)
const kugouPlaylistStatusLabel = computed(() => {
  if (kugouPlaylistError.value) {
    return kugouPlaylistError.value
  }

  if (kugouSelectedContentSource.value === 'recommended' && kugouRecommendedPlaylistsLoading.value) {
    return '正在读取推荐歌单...'
  }

  if (!kugouLoggedIn.value && kugouSelectedContentSource.value !== 'recommended') {
    return '登录后可以读取你的酷狗个人歌单。'
  }

  if (kugouPlaylistsLoading.value) {
    return '正在读取个人歌单...'
  }

  if (kugouPlaylistDetailLoading.value) {
    return '正在读取歌单歌曲...'
  }

  if (kugouPlaylistDetail.value) {
    return kugouPlaylistDetail.value.message
  }

  if (kugouSelectedContentSource.value === 'recommended' && kugouRecommendedPlaylists.value.length > 0) {
    return '选择一个推荐歌单查看歌曲摘要。'
  }

  if (kugouPlaylists.value.length > 0) {
    return '选择一个歌单查看歌曲摘要。'
  }

  return kugouSelectedContentSource.value === 'recommended'
    ? '点击刷新推荐获取酷狗推荐歌单。'
    : '点击读取歌单获取酷狗个人歌单列表。'
})
const kugouPlaylistHasMore = computed(() =>
  Boolean(
    kugouPlaylistDetail.value?.truncated &&
      kugouPlaylistDetailPage.value > 0 &&
      kugouPlaylistDetailPage.value < MAX_PLATFORM_PLAYLIST_PAGE,
  ),
)
const kugouPlaylistLoadedLabel = computed(() => {
  const detail = kugouPlaylistDetail.value
  if (!detail) {
    return '0 首'
  }

  return detail.totalTrackCount > 0
    ? `${detail.tracks.length} / ${detail.totalTrackCount} 首`
    : `${detail.tracks.length} 首`
})
const kugouSearchTracks = computed(() => kugouSearchResult.value?.tracks ?? [])
const kugouDailyRecommendationTracks = computed(() => kugouDailyRecommendation.value?.tracks ?? [])
const kugouSearchHasMore = computed(() => {
  const result = kugouSearchResult.value
  return Boolean(
    result &&
      result.total > result.tracks.length &&
      kugouSearchPage.value > 0 &&
      kugouSearchPage.value < MAX_PLATFORM_SEARCH_PAGE,
  )
})
const kugouSearchLoadedLabel = computed(() => {
  const result = kugouSearchResult.value
  if (!result) {
    return '输入关键词后搜索'
  }

  return result.total > 0
    ? `${result.tracks.length} / ${result.total} 首`
    : `${result.tracks.length} 首`
})
const kugouDailyRecommendationLoadedLabel = computed(() => {
  const result = kugouDailyRecommendation.value
  if (!result) {
    return '点击读取每日推荐'
  }

  return result.total > 0
    ? `${result.tracks.length} / ${result.total} 首`
    : `${result.tracks.length} 首`
})
const kugouStatusLabel = computed(() => {
  return kugouLoginStatusLabel.value
})
const kugouStatusDetail = computed(() => {
  return kugouLoginStatusDetail.value
})
const kugouSearchStatusDetail = computed(() => {
  if (kugouSearchError.value) {
    return kugouSearchError.value
  }

  if (kugouSearchNotice.value) {
    return kugouSearchNotice.value
  }

  return '输入关键词搜索酷狗歌曲；搜索结果只作为临时播放列表，不写入本机曲库。'
})
const activeImmersiveSearchPlatforms = computed(() =>
  IMMERSIVE_SEARCH_PLATFORMS.filter((platform) => immersiveSearchPlatforms.value.includes(platform)),
)
const immersiveSearchPlatformLabel = computed(() => {
  const platforms = activeImmersiveSearchPlatforms.value
  if (platforms.length === IMMERSIVE_SEARCH_PLATFORMS.length) {
    return '全部平台'
  }

  return platforms.map(immersiveSearchPlatformName).join('、') || '未选择平台'
})
const immersiveSearchPlaceholder = computed(
  () => `搜索${immersiveSearchPlatformLabel.value}歌曲或歌手`,
)
const immersiveNeteaseSearchTracks = computed(() => immersiveNeteaseSearchResult.value?.tracks ?? [])
const immersiveKugouSearchTracks = computed(() => immersiveKugouSearchResult.value?.tracks ?? [])
const immersiveSearchResults = computed<ImmersiveSearchResultItem[]>(() => {
  const includeNetease = activeImmersiveSearchPlatforms.value.includes('netease')
  const includeKugou = activeImmersiveSearchPlatforms.value.includes('kugou')
  const neteaseTracks = includeNetease ? immersiveNeteaseSearchTracks.value : []
  const kugouTracks = includeKugou ? immersiveKugouSearchTracks.value : []
  const results: ImmersiveSearchResultItem[] = []
  const maxLength = Math.max(neteaseTracks.length, kugouTracks.length)

  for (let index = 0; index < maxLength; index += 1) {
    const neteaseTrack = neteaseTracks[index]
    if (neteaseTrack) {
      results.push({
        key: `netease-${neteaseTrack.id}`,
        platform: 'netease',
        track: neteaseTrack,
        sourceIndex: index,
      })
    }

    const kugouTrack = kugouTracks[index]
    if (kugouTrack) {
      results.push({
        key: `kugou-${kugouTrack.hash || kugouTrack.id}-${index}`,
        platform: 'kugou',
        track: kugouTrack,
        sourceIndex: index,
      })
    }
  }

  return results
})
const immersiveSearchPanelVisible = computed(
  () => immersiveSearchFocused.value || immersiveSearchLoading.value,
)
const immersiveSearchStatusLabel = computed(() => {
  if (immersiveSearchError.value) {
    return immersiveSearchError.value
  }

  if (immersiveSearchLoading.value) {
    return `正在搜索${immersiveSearchPlatformLabel.value}...`
  }

  if (!immersiveSearchExecuted.value) {
    return '输入关键词后搜索'
  }

  if (immersiveSearchResults.value.length === 0) {
    return `没有搜索到“${immersiveSearchQuery.value.trim()}”。`
  }

  if (immersiveSearchNotice.value) {
    return immersiveSearchNotice.value
  }

  return `已找到 ${immersiveSearchResults.value.length} 首歌曲`
})
const kugouDailyRecommendationStatusDetail = computed(() => {
  if (kugouDailyRecommendationError.value) {
    return kugouDailyRecommendationError.value
  }

  if (kugouDailyRecommendationNotice.value) {
    return kugouDailyRecommendationNotice.value
  }

  return '每日推荐只作为当前运行态在线列表，不写入本机曲库。'
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
const repeatModeIcon = computed(() => {
  if (repeatMode.value === 'one') {
    return '①'
  }

  if (repeatMode.value === 'all') {
    return '↻'
  }

  return '→'
})
const visualModeLabel = computed(
  () => visualModeOptions.find((option) => option.value === visualMode.value)?.label ?? '韵律',
)
const visualStagePresetOption = computed(
  () => stagePresetOptions.find((option) => option.value === visualStagePreset.value) ?? defaultVisualStagePresetOption,
)
const visualStagePresetLabel = computed(() => visualStagePresetOption.value.label)
const visualStagePresetDetail = computed(
  () => `${visualStagePresetOption.value.kicker} · ${visualStagePresetOption.value.metrics.join(' · ')}`,
)
const visualSpectrumStyleLabel = computed(
  () => spectrumStyleOptions.find((option) => option.value === visualSpectrumStyle.value)?.label ?? '竖条',
)
const webglStarfieldActive = computed(
  () => immersiveMode.value && !webglStarfieldUnavailable.value,
)
const canvasSpectrumStyle = computed<MusicSpectrumStyle>(() =>
  webglStarfieldActive.value ? 'none' : visualSpectrumStyle.value,
)
const canvasLineStyle = computed<MusicLineStyle>(() =>
  webglStarfieldActive.value ? 'none' : visualLineStyle.value,
)
const canvasRippleStyle = computed<MusicRippleStyle>(() =>
  webglStarfieldActive.value ? 'none' : visualRippleStyle.value,
)
const canvasForegroundDisabled = computed(() => webglStarfieldActive.value)
const visualLineStyleLabel = computed(
  () => lineStyleOptions.find((option) => option.value === visualLineStyle.value)?.label ?? '柔波',
)
const visualRippleStyleLabel = computed(
  () => rippleStyleOptions.find((option) => option.value === visualRippleStyle.value)?.label ?? '圆环',
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
const currentTrackOnline = computed(() =>
  currentTrack.value?.source === 'netease' || currentTrack.value?.source === 'kugou',
)
const currentTrackPlatformLabel = computed(() => {
  if (currentTrack.value?.source === 'netease') {
    return '网易云在线音乐'
  }

  if (currentTrack.value?.source === 'kugou') {
    return '酷狗在线音乐'
  }

  return '本机音乐'
})
const playbackListTrackCount = computed(() => {
  if (currentTrack.value?.source === 'netease') {
    return currentNeteasePlaybackTracks().length
  }

  if (currentTrack.value?.source === 'kugou') {
    return currentKugouPlaybackTracks().length
  }

  return localPlaybackTrackIds().length
})
const immersiveLocalPlaylistTracks = computed(() => {
  const sourceTracks = currentLocalImmersiveSourceTracks()
  const currentTrackId =
    currentTrack.value?.source === 'local' || !currentTrack.value?.source
      ? currentTrack.value?.id ?? ''
      : ''
  const currentListIndex = sourceTracks.findIndex((track) => track.id === currentTrackId)
  return centeredPlaybackWindow(sourceTracks, currentListIndex)
})
const immersiveNeteasePlaylistTracks = computed(() => {
  const sourceTracks = currentNeteaseImmersiveSourceTracks()
  const currentSongId =
    currentTrack.value?.source === 'netease' ? currentTrack.value.neteaseSongId : null
  const currentListIndex = sourceTracks.findIndex((track) => track.id === currentSongId)
  return centeredPlaybackWindow(sourceTracks, currentListIndex)
})
const immersiveKugouPlaylistTracks = computed(() => {
  const sourceTracks = currentKugouImmersiveSourceTracks()
  const currentHash =
    currentTrack.value?.source === 'kugou' ? currentTrack.value.kugouSongHash : ''
  const currentListIndex = sourceTracks.findIndex((track) => track.hash === currentHash)
  return centeredPlaybackWindow(sourceTracks, currentListIndex)
})
const immersivePlaylistCountLabel = computed(() => {
  if (immersivePlaylistSource.value === 'netease') {
    if (!neteaseLoggedIn.value) {
      return '未登录网易云'
    }

    if (neteasePlaylistDetailLoading.value) {
      return '正在读取网易云歌单'
    }

    const playlistName =
      neteaseSearchTracks.value.length > 0
        ? `网易云搜索：${neteaseSearchResult.value?.keyword ?? ''}`.trim()
        : neteaseSelectedPlaylist.value?.name ?? '网易云歌单'
    const count = immersiveNeteasePlaylistTracks.value.length
    const total = currentNeteaseImmersiveSourceTracks().length
    return count > 0 ? `${playlistName} · 当前附近 ${count} / ${total} 首` : '等待读取网易云歌单'
  }

  if (immersivePlaylistSource.value === 'kugou') {
    if (kugouPlaylistDetailLoading.value) {
      return '正在读取酷狗歌单'
    }

    if (kugouSearchLoading.value) {
      return '正在搜索酷狗'
    }

    if (kugouDailyRecommendationLoading.value) {
      return '正在读取酷狗每日推荐'
    }

    const count = immersiveKugouPlaylistTracks.value.length
    const total = currentKugouImmersiveSourceTracks().length
    if (count > 0 && kugouPlaylistDetail.value) {
      return `${kugouPlaylistDetail.value.playlist.name} · 当前附近 ${count} / ${total} 首`
    }

    if (count > 0 && kugouDailyRecommendationTracks.value.length > 0) {
      return `酷狗每日推荐 · 当前附近 ${count} / ${total} 首`
    }

    return count > 0 ? `酷狗搜索 · 当前附近 ${count} / ${total} 首` : '等待酷狗搜索或歌单'
  }

  if (currentTrackOnline.value) {
    return currentTrack.value?.source === 'netease' && playQueue.value.length > 0
      ? `网易云临时播放 · 队列 ${playQueue.value.length}`
      : `${currentTrackPlatformLabel.value}临时播放`
  }

  const count = immersiveLocalPlaylistTracks.value.length
  const total = currentLocalImmersiveSourceTracks().length
  return count > 0 ? `当前附近 ${count} / ${total} 首` : '等待本机歌曲'
})
const immersivePlaylistEmptyLabel = computed(() => {
  if (immersivePlaylistSource.value === 'netease') {
    if (!neteaseLoggedIn.value) {
      return '先登录网易云音乐'
    }

    if (neteasePlaylistDetailLoading.value) {
      return '正在读取网易云歌单'
    }

    return '先在网易云页搜索歌曲或读取歌单'
  }

  if (immersivePlaylistSource.value === 'kugou') {
    if (kugouPlaylistDetailLoading.value) {
      return '正在读取酷狗歌单'
    }

    if (kugouSearchLoading.value) {
      return '正在搜索酷狗歌曲'
    }

    if (kugouDailyRecommendationLoading.value) {
      return '正在读取酷狗每日推荐'
    }

    return '先在酷狗页搜索歌曲、读取歌单或每日推荐'
  }

  return '暂无歌曲'
})
const immersiveLyrics = computed(() => lyricsAt(visualLyricsTime.value, currentTrack.value))
const immersiveLyricsStatusLabel = computed(() => {
  if (lyricsStatus.value === 'loading') {
    return currentTrackOnline.value ? '在线歌词读取中' : '本机歌词读取中'
  }

  if (lyricsStatus.value === 'ready') {
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
const immersiveLyricsLayoutStyle = computed(() => {
  const nextLine =
    lyricsStatus.value === 'error' ? lyricsError.value : immersiveLyrics.value.next
  return resolveImmersiveLyricsLayoutStyle(
    immersiveLyrics.value.current,
    immersiveLyrics.value.previous,
    nextLine,
  )
})
const immersiveLyricsStageStyle = computed(() => ({
  ...immersiveLyricsLayoutStyle.value,
  ...resolveImmersiveLyricStageStyle(),
}))
const webglLyricStage = computed<WebglLyricStageState>(() => {
  const lyric = immersiveLyrics.value
  const nextLine = lyricsStatus.value === 'error' ? lyricsError.value : lyric.next
  const fontScale = lyricFontScaleValue()
  const lineCapacityScale = clamp(1 / fontScale, 0.72, 1.38)
  const active =
    webglStarfieldActive.value &&
    Boolean(currentTrack.value)
  const fallbackArtist = currentTrack.value?.artist.trim() || ''
  const currentLyricText = lyric.current.trim()
  const readyLyric = lyricsStatus.value === 'ready' && currentLyricText.length > 0
  const textMode: WebglLyricStageState['textMode'] =
    !currentTrack.value ? 'placeholder'
      : lyricsStatus.value === 'loading' ? 'loading'
        : lyricsStatus.value === 'error' ? 'error'
          : lyricsStatus.value === 'empty' ? 'empty'
            : readyLyric ? 'lyric'
              : 'placeholder'
  const statusText = immersiveLyricsStatusLabel.value
  const currentText =
    textMode === 'lyric'
      ? currentLyricText
      : textMode === 'loading'
        ? ''
        : textMode === 'error'
          ? '歌词不可用'
          : textMode === 'empty'
            ? '未找到歌词'
            : ''
  const previousText =
    textMode === 'lyric'
      ? lyric.previous
      : ''
  const nextText =
    textMode === 'error'
      ? lyricsError.value || fallbackArtist
      : textMode === 'empty'
        ? fallbackArtist || '可继续播放'
        : textMode === 'loading'
          ? ''
          : textMode === 'lyric'
            ? nextLine
            : ''
  const currentLines = estimateImmersiveLyricLineCount(
    currentText,
    Math.max(9, Math.round(14 * lineCapacityScale)),
    IMMERSIVE_LYRIC_MAIN_MAX_LINES,
  )
  const sideLines = Math.max(
    estimateImmersiveLyricLineCount(
      previousText,
      Math.max(16, Math.round(24 * lineCapacityScale)),
      IMMERSIVE_LYRIC_SIDE_MAX_LINES,
    ),
    estimateImmersiveLyricLineCount(
      nextText,
      Math.max(16, Math.round(24 * lineCapacityScale)),
      IMMERSIVE_LYRIC_SIDE_MAX_LINES,
    ),
  )
  const trackKey = currentTrack.value?.id ?? 'no-track'
  const stageLyricKeyPrefix = `${trackKey}:${textMode}:${lyricsStatus.value}`

  return {
    active,
    textMode,
    statusText: textMode === 'empty' || textMode === 'error' ? statusText : '',
    current: active ? currentText : '',
    currentKey: active ? `${stageLyricKeyPrefix}:current:${lyric.currentKey}:${currentText}` : 'inactive-current',
    previous: active ? previousText : '',
    previousKey: active ? `${stageLyricKeyPrefix}:previous:${lyric.previousKey}:${previousText}` : 'inactive-previous',
    next: active ? nextText : '',
    nextKey: active ? `${stageLyricKeyPrefix}:next:${lyric.nextKey}:${nextText}` : 'inactive-next',
    progress: clamp(lyric.progress, 0, 1),
    status: lyricsStatus.value,
    synced: lyric.synced,
    interlude: lyric.interlude,
    fontScale,
    tilt: clamp(lyricStageTilt.value, 0, 1),
    glow: clamp(lyricStageGlow.value, 0, 1),
    verticalOffsetPx: lyricVerticalOffsetPx(),
    distanceOffsetPx: lyricDistanceOffsetPx(),
    distanceScale: lyricDistanceScaleValue(),
    sideOpacity: lyricSideOpacityValue(),
    currentLines,
    sideLines,
  }
})
const lyricOffsetLabel = computed(() => {
  const value = Math.round(lyricOffsetMs.value)
  if (value === 0) {
    return '0 ms'
  }

  return value > 0 ? `提前 ${value} ms` : `延后 ${Math.abs(value)} ms`
})
const lyricStageTiltLabel = computed(() => `${Math.round(clamp(lyricStageTilt.value, 0, 1) * 100)}%`)
const lyricStageGlowLabel = computed(() => `${Math.round(clamp(lyricStageGlow.value, 0, 1) * 100)}%`)
const lyricStageFontScaleLabel = computed(() => `${Math.round(lyricFontScaleValue() * 100)}%`)
const lyricStageVerticalLabel = computed(() => {
  const value = lyricVerticalOffsetPx()
  if (value === 0) {
    return '0px'
  }

  return value > 0 ? `下 ${value}px` : `上 ${Math.abs(value)}px`
})
const lyricStageDistanceLabel = computed(() => {
  const value = lyricDistanceOffsetPx()
  if (value === 0) {
    return '默认'
  }

  return value > 0 ? `近 ${value}px` : `远 ${Math.abs(value)}px`
})
const lyricStageSideOpacityLabel = computed(() => `${Math.round(lyricSideOpacityValue() * 100)}%`)
const visualizerStatusLabel = computed(() => {
  if (!currentTrack.value) {
    return '待机'
  }

  if (beatMapStatus.value === 'analyzing') {
    return `节奏分析中 ${Math.round(beatMapProgress.value * 100)}%`
  }

  if (beatMapMatchesCurrentTrack.value) {
    return playing.value
      ? `${visualStagePresetLabel.value} · ${visualStagePresetDetail.value}`
      : '已暂停'
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

  return playing.value
    ? `${visualStagePresetLabel.value} · ${visualStagePresetDetail.value}`
    : '已暂停'
})
const visualizerEnergyLabel = computed(() => {
  const energy = visualEnergyFrame.value
  const value = Math.round(clamp((energy.bass * 0.45 + energy.mid * 0.35 + energy.treble * 0.2) * 100, 0, 100))
  return `${value}%`
})
const immersiveStageStyle = computed(() => {
  const energy = visualEnergyFrame.value
  const motionScale = visualReducedMotion.value ? 0.38 : 1
  const audioLift = playing.value ? clamp(energy.beat * 0.7 + energy.bass * 0.42, 0, 1) : 0
  const depth = Math.round(audioLift * 18 * motionScale)
  const scale = 1 + audioLift * 0.012 * motionScale
  const glow = 0.42 + clamp(energy.volume * 0.32 + energy.beat * 0.24, 0, 0.46) * motionScale
  const cssDepthScale = webglStarfieldActive.value ? 0.32 : 1

  return {
    '--immersive-stage-yaw': `${(immersiveStageYaw.value * cssDepthScale).toFixed(2)}deg`,
    '--immersive-stage-pitch': `${(immersiveStagePitch.value * cssDepthScale).toFixed(2)}deg`,
    '--immersive-stage-z': `${depth}px`,
    '--immersive-stage-scale': scale.toFixed(4),
    '--immersive-stage-glow': glow.toFixed(3),
  }
})
const immersiveFreeCameraView = computed<ImmersiveFreeCameraView>(() => ({
  active: immersiveFreeCameraActive.value,
  locked: immersiveFreeCameraLocked.value || immersiveFreeCameraResetting.value,
  x: immersiveFreeCameraX.value,
  y: immersiveFreeCameraY.value,
  z: immersiveFreeCameraZ.value,
  yaw: immersiveFreeCameraYaw.value,
  pitch: immersiveFreeCameraPitch.value,
  roll: immersiveFreeCameraRoll.value,
  fov: immersiveFreeCameraFov.value,
}))
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
let unlistenMusicImmersiveThemeChanged: (() => void) | null = null
let unlistenMusicAction: (() => void) | null = null
let unlistenMusicWindowMoved: (() => void) | null = null
let playlistsRestored = false
let beatMapRequestedTrackId = ''
let lyricsRequestedTrackId = ''
let visualClockFrameId: number | null = null
let lastVisualClockUpdate = 0
let lastLyricMusicEnvelopeUpdate = 0
let playbackRequestSerial = 0
let onlinePlaybackPrefetchTimer: number | null = null
let onlinePlaybackPrefetchSerial = 0
let onlineStallRecoveryTimer: number | null = null
let onlineStallRecoverySerial = 0
let onlineStallRecoveryAttempts = 0
let onlineStallStartedAt = 0
let immersiveContentPrepTimer: number | null = null
let immersiveContentPrepSerial = 0
let immersiveStagePointerId: number | null = null
let immersiveStageLastX = 0
let immersiveStageLastY = 0
let immersiveStageMomentumFrameId: number | null = null
let immersiveFreeCameraFrameId: number | null = null
let immersiveFreeCameraLastFrame = 0
let immersiveFreeCameraVelocityX = 0
let immersiveFreeCameraVelocityY = 0
let immersiveFreeCameraVelocityZ = 0
let immersiveFreeCameraPointerSeen = false
let immersiveFreeCameraPointerX = 0
let immersiveFreeCameraPointerY = 0
let immersiveFreeCameraResetStart = 0
let immersiveFreeCameraResetFrom = { ...IMMERSIVE_FREE_CAMERA_DEFAULT }
const immersiveFreeCameraKeys = new Set<string>()
let miniEdgeDockState: MiniEdgeDockState | null = null
let miniEdgeMoveTimer: number | null = null
let miniEdgeHideTimer: number | null = null
let miniEdgeDragPollTimer: number | null = null
let miniEdgeDragPollStartedAt = 0
let miniEdgeDragPollLastPosition: { x: number; y: number } | null = null
let miniEdgeDragPollIdleTicks = 0
let miniEdgeDragPollSeenMove = false
let miniEdgeInteractionSerial = 0
let miniEdgeWindowMoveSerial = 0
let suppressMiniEdgeMoveUntil = 0
let neteaseQrPollTimer: number | null = null
let kugouQrPollTimer: number | null = null
let immersiveSearchRequestSerial = 0

onMounted(async () => {
  restoreSettings()
  restoreTracks()
  restorePlaylists()
  syncAudioVolume()
  await loadTheme()
  void refreshNeteaseLoginStatus(false)
  void refreshKugouLoginStatus(false)
  window.addEventListener('keydown', handleImmersiveFreeCameraKeyDown, true)
  window.addEventListener('keyup', handleImmersiveFreeCameraKeyUp, true)
  window.addEventListener('mousemove', handleImmersiveFreeCameraMouseMove, true)
  window.addEventListener('pointerdown', handleImmersiveSearchDocumentPointerDown, true)
  unlistenThemeChanged = await listen<string>('ui-theme-changed', (event) => {
    drawerTheme.value = normalizeDrawerTheme(event.payload)
  })
  unlistenMusicImmersiveThemeChanged = await listen<string>('ui-music-immersive-theme-changed', (event) => {
    musicImmersiveThemePreference.value = normalizeMusicImmersiveTheme(event.payload)
  })
  unlistenMusicAction = await listen<MusicActionRequest>('music-action-requested', (event) => {
    void handleMusicActionRequest(event.payload)
  })
  try {
    unlistenMusicWindowMoved = await musicWindow.onMoved(() => {
      scheduleMiniEdgeDockCheck()
    })
  } catch {
    unlistenMusicWindowMoved = null
  }
})

onBeforeUnmount(() => {
  unlistenThemeChanged?.()
  unlistenMusicImmersiveThemeChanged?.()
  unlistenMusicAction?.()
  unlistenMusicWindowMoved?.()
  window.removeEventListener('keydown', handleImmersiveFreeCameraKeyDown, true)
  window.removeEventListener('keyup', handleImmersiveFreeCameraKeyUp, true)
  window.removeEventListener('mousemove', handleImmersiveFreeCameraMouseMove, true)
  window.removeEventListener('pointerdown', handleImmersiveSearchDocumentPointerDown, true)
  clearMiniEdgeTimers()
  cancelMiniEdgeWindowMove()
  clearOnlinePlaybackPrefetchTimer()
  clearOnlineStallRecoveryTimer()
  clearImmersiveContentPrepTimer()
  clearImmersiveStageMomentum()
  clearImmersiveFreeCamera()
  stopVisualClock()
  stopNeteaseQrPolling()
  stopKugouQrPolling()
})

watch(volume, () => {
  syncAudioVolume()
  saveSettings()
})

watch(playerError, () => {
  playerErrorDetailOpen.value = false
})

watch(
  [
    repeatMode,
    shuffleEnabled,
    musicStorageDir,
    onlinePlaybackQuality,
    neteaseOnlinePlaybackQuality,
    kugouOnlinePlaybackQuality,
    importCategory,
    visualStagePreset,
    visualMode,
    visualSpectrumStyle,
    visualLineStyle,
    visualRippleStyle,
    visualIntensity,
    visualReducedMotion,
    lyricStageTilt,
    lyricStageGlow,
    lyricStageFontScale,
    lyricStageVertical,
    lyricStageDistance,
    lyricStageSideOpacity,
    lyricOffsetMs,
  ],
  saveSettings,
)

watch([onlinePlaybackQuality, neteaseOnlinePlaybackQuality, kugouOnlinePlaybackQuality], () => {
  clearOnlinePlaybackRuntimeCache()
})

watch(
  visualStageTunings,
  () => {
    saveSettings()
  },
  { deep: true },
)

watch(visualReducedMotion, (reducedMotion) => {
  if (reducedMotion) {
    resetLyricMusicEnvelope()
  }
})

watch(() => currentTrack.value?.id ?? '', () => {
  invalidatePendingPlayback()
  clearOnlinePlaybackPrefetchTimer()
  resetOnlineStallRecovery()
  clearImmersiveContentPrepTimer()
  beatMapRequestedTrackId = ''
  lyricsRequestedTrackId = ''
  visualPlaybackTime.value = 0
  resetLyricMusicEnvelope()
  resetEnergyFrame()
  resetBeatMap()
  resetLyrics()
  if (immersiveMode.value && currentTrack.value) {
    scheduleImmersiveContentPreparation()
  }
})

watch(() => currentKugouQualityTrack.value?.hash ?? '', () => {
  const track = currentKugouQualityTrack.value
  if (track) {
    void ensureKugouQualityAvailability(track).catch(() => {})
  }
})

watch(activePanelView, (view) => {
  if (view === 'netease') {
    stopKugouQrPolling()
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

  if (view === 'kugou') {
    void (async () => {
      await refreshKugouLoginStatus(false)
      if (
        kugouLoggedIn.value &&
        kugouPlaylists.value.length === 0 &&
        !kugouPlaylistsLoading.value
      ) {
        await refreshKugouPlaylists(false)
      }
      if (
        kugouRecommendedPlaylists.value.length === 0 &&
        !kugouRecommendedPlaylistsLoading.value
      ) {
        await refreshKugouRecommendedPlaylists(false)
      }
    })()
    stopNeteaseQrPolling()
    return
  }

  stopNeteaseQrPolling()
  stopKugouQrPolling()
})

watch(playing, (isPlaying) => {
  if (isPlaying && immersiveMode.value) {
    void prepareImmersiveVisualization()
    startVisualClock()
  } else {
    clearImmersiveContentPrepTimer()
    resetImmersiveStageView()
    resetLyricMusicEnvelope()
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
      onlinePlaybackQuality?: string
      neteaseOnlinePlaybackQuality?: string
      kugouOnlinePlaybackQuality?: string
      importCategory?: string
      playQueue?: string[]
      visualStagePreset?: MusicVisualStagePreset
      visualMode?: MusicVisualMode
      visualSpectrumStyle?: MusicSpectrumStyle
      visualLineStyle?: MusicLineStyle
      visualRippleStyle?: MusicRippleStyle
      visualIntensity?: number
      visualReducedMotion?: boolean
      visualStageTuning?: Partial<MusicStageTuning>
      visualStageTunings?: Partial<Record<MusicVisualStagePreset, Partial<MusicStageTuning>>>
      lyricStageTilt?: number
      lyricStageGlow?: number
      lyricStageFontScale?: number
      lyricStageVertical?: number
      lyricStageDistance?: number
      lyricStageSideOpacity?: number
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
    const restoredOnlinePlaybackQuality = normalizeOnlinePlaybackQuality(saved.onlinePlaybackQuality)
    onlinePlaybackQuality.value = normalizeOnlinePlaybackQualityForPlatform(
      restoredOnlinePlaybackQuality,
      'general',
    )
    neteaseOnlinePlaybackQuality.value = normalizeOnlinePlaybackQualityForPlatform(
      saved.neteaseOnlinePlaybackQuality ?? restoredOnlinePlaybackQuality,
      'netease',
    )
    kugouOnlinePlaybackQuality.value = normalizeOnlinePlaybackQualityForPlatform(
      saved.kugouOnlinePlaybackQuality ?? restoredOnlinePlaybackQuality,
      'kugou',
    )
    importCategory.value = normalizeMusicCategory(saved.importCategory)
    playQueue.value = normalizeQueueIds(saved.playQueue)
    if (isMusicVisualMode(saved.visualMode)) {
      visualMode.value = saved.visualMode
    }
    if (isMusicSpectrumStyle(saved.visualSpectrumStyle)) {
      visualSpectrumStyle.value = saved.visualSpectrumStyle
    }
    if (isMusicLineStyle(saved.visualLineStyle)) {
      visualLineStyle.value = saved.visualLineStyle
    }
    if (isMusicRippleStyle(saved.visualRippleStyle)) {
      visualRippleStyle.value = saved.visualRippleStyle
    }
    if (isMusicVisualStagePreset(saved.visualStagePreset)) {
      applyVisualStagePreset(saved.visualStagePreset)
    }
    if (typeof saved.visualIntensity === 'number') {
      visualIntensity.value = clamp(saved.visualIntensity, 0.2, 1)
    }
    visualReducedMotion.value = Boolean(saved.visualReducedMotion)
    visualStageTunings.value = normalizeMusicStageTunings(saved.visualStageTunings, saved.visualStageTuning)
    const lyricDefaults = lyricStageDefaultsForPreset('projection')
    lyricStageTilt.value = normalizeUnitSetting(saved.lyricStageTilt, lyricDefaults.tilt)
    lyricStageGlow.value = normalizeUnitSetting(saved.lyricStageGlow, lyricDefaults.glow)
    lyricStageFontScale.value = normalizeUnitSetting(saved.lyricStageFontScale, lyricDefaults.fontScale)
    lyricStageVertical.value = normalizeUnitSetting(saved.lyricStageVertical, lyricDefaults.vertical)
    lyricStageDistance.value = normalizeUnitSetting(saved.lyricStageDistance, LYRIC_STAGE_DISTANCE_DEFAULT)
    lyricStageSideOpacity.value = normalizeUnitSetting(saved.lyricStageSideOpacity, lyricDefaults.sideOpacity)
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
      coverImgUrl?: string | null
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
          coverImgUrl?: string | null
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
            coverImgUrl: normalizeCoverImgUrl(item.coverImgUrl),
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
      onlinePlaybackQuality: onlinePlaybackQuality.value,
      neteaseOnlinePlaybackQuality: neteaseOnlinePlaybackQuality.value,
      kugouOnlinePlaybackQuality: kugouOnlinePlaybackQuality.value,
      importCategory: normalizeMusicCategory(importCategory.value),
      playQueue: sanitizeQueueIds(playQueue.value),
      visualStagePreset: visualStagePreset.value,
      visualMode: visualMode.value,
      visualSpectrumStyle: visualSpectrumStyle.value,
      visualLineStyle: visualLineStyle.value,
      visualRippleStyle: visualRippleStyle.value,
      visualIntensity: clamp(visualIntensity.value, 0.2, 1),
      visualReducedMotion: visualReducedMotion.value,
      visualStageTunings: normalizeMusicStageTunings(visualStageTunings.value),
      lyricStageTilt: normalizeUnitSetting(lyricStageTilt.value, LYRIC_STAGE_PRESET_DEFAULTS.projection.tilt),
      lyricStageGlow: normalizeUnitSetting(lyricStageGlow.value, LYRIC_STAGE_PRESET_DEFAULTS.projection.glow),
      lyricStageFontScale: normalizeUnitSetting(
        lyricStageFontScale.value,
        LYRIC_STAGE_PRESET_DEFAULTS.projection.fontScale,
      ),
      lyricStageVertical: normalizeUnitSetting(
        lyricStageVertical.value,
        LYRIC_STAGE_PRESET_DEFAULTS.projection.vertical,
      ),
      lyricStageDistance: normalizeUnitSetting(lyricStageDistance.value, LYRIC_STAGE_DISTANCE_DEFAULT),
      lyricStageSideOpacity: normalizeUnitSetting(
        lyricStageSideOpacity.value,
        LYRIC_STAGE_PRESET_DEFAULTS.projection.sideOpacity,
      ),
      lyricOffsetMs: clamp(Math.round(lyricOffsetMs.value), -2000, 2000),
    }),
  )
}

function saveTracks() {
  localStorage.setItem(
    TRACKS_STORAGE_KEY,
    JSON.stringify(
      tracks.value
        .filter((track) => track.source !== 'netease' && track.source !== 'kugou')
        .map((track) => ({
          id: track.id,
          path: track.path,
          sourcePath: track.sourcePath,
          title: track.title,
          artist: normalizeTrackArtist(track.artist),
          album: normalizeTrackAlbum(track.album),
          coverImgUrl: normalizeCoverImgUrl(track.coverImgUrl),
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
    drawerTheme.value = normalizeDrawerTheme(config.drawer.theme)
    musicImmersiveThemePreference.value = normalizeMusicImmersiveTheme(config.drawer.musicImmersiveTheme)
  } catch {
    drawerTheme.value = 'light'
    musicImmersiveThemePreference.value = 'follow'
  }
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

async function setMusicImmersiveThemePreference(theme: MusicImmersiveThemePreference) {
  if (immersiveThemeSaving.value || musicImmersiveThemePreference.value === theme) {
    return
  }

  const previous = musicImmersiveThemePreference.value
  immersiveThemeSaving.value = true
  immersiveThemeError.value = ''
  musicImmersiveThemePreference.value = theme

  try {
    const config = await invoke<PetDrawerConfig>('save_music_immersive_theme', { theme })
    const savedTheme = normalizeMusicImmersiveTheme(config.drawer.musicImmersiveTheme)
    musicImmersiveThemePreference.value = savedTheme
    void emitEvent('ui-music-immersive-theme-changed', savedTheme)
  } catch (err) {
    musicImmersiveThemePreference.value = previous
    immersiveThemeError.value = String(err)
  } finally {
    immersiveThemeSaving.value = false
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
    | 'coverImgUrl'
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
    coverImgUrl: normalizeCoverImgUrl(playback?.coverImgUrl),
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

function normalizeOnlinePlaybackQuality(value?: string | null): OnlinePlaybackQuality {
  const normalized = value?.trim().toLowerCase()
  return ALL_ONLINE_PLAYBACK_QUALITY_OPTIONS.some((option) => option.value === normalized)
    ? (normalized as OnlinePlaybackQuality)
    : DEFAULT_ONLINE_PLAYBACK_QUALITY
}

function onlinePlaybackQualityOptionsForPlatform(platform: OnlinePlaybackQualityPlatform) {
  if (platform === 'netease') {
    return NETEASE_ONLINE_PLAYBACK_QUALITY_OPTIONS
  }
  if (platform === 'kugou') {
    return KUGOU_ONLINE_PLAYBACK_QUALITY_OPTIONS
  }
  return GENERAL_ONLINE_PLAYBACK_QUALITY_OPTIONS
}

function onlinePlaybackQualityViewOptionsForPlatform(platform: OnlinePlaybackQualityPlatform) {
  const options = onlinePlaybackQualityOptionsForPlatform(platform)
  if (platform !== 'kugou') {
    return options
  }

  return options.map((option) => {
    const availability = kugouQualityAvailabilityForOption(option.value)
    if (!availability) {
      return option
    }

    const reason = availability.reason?.trim() || ''
    const statusLabel = kugouQualityAvailabilityStatusLabel(availability.status)
    return {
      ...option,
      disabled: availability.status === 'unavailable',
      availabilityStatus: availability.status,
      availabilityReason: reason,
      availabilityDetail: availability.detail?.trim() || '',
      description:
        availability.status === 'available'
          ? `${option.description} · ${statusLabel}`
          : reason
            ? `${option.description} · ${statusLabel}：${reason}`
            : `${option.description} · ${statusLabel}`,
    }
  })
}

function kugouQualityAvailabilityStatusLabel(status: KugouQualityAvailabilityStatus) {
  if (status === 'available') {
    return '当前歌曲可用'
  }
  if (status === 'unavailable') {
    return '当前歌曲不可用'
  }
  return '未确认'
}

function kugouQualityAvailabilityForOption(value: OnlinePlaybackQuality) {
  const availability = currentKugouQualityAvailability.value
  if (!availability) {
    return null
  }

  return availability.qualities.find((item) => item.quality === value) ?? null
}

function onlinePlaybackQualityOptionDisabled(option: OnlinePlaybackQualityOption) {
  return Boolean(option.disabled)
}

function onlinePlaybackQualityOptionTitle(option: OnlinePlaybackQualityOption) {
  const status = option.availabilityStatus
  const reason = option.availabilityReason?.trim()
  const detail = option.availabilityDetail?.trim()
  const parts = [`${option.label}：${option.description}`]
  if (status) {
    parts.push(kugouQualityAvailabilityStatusLabel(status))
  }
  if (reason) {
    parts.push(reason)
  }
  if (detail) {
    parts.push(detail)
  }
  return parts.join('；')
}

function onlinePlaybackQualityPlatformText(platform: OnlinePlaybackQualityPlatform) {
  if (platform === 'netease') {
    return '网易云音质'
  }
  if (platform === 'kugou') {
    return '酷狗音质'
  }
  return '在线播放音质'
}

function currentOnlinePlaybackQualityPlatform(): OnlinePlaybackQualityPlatform {
  const source = currentTrack.value?.source
  if (source === 'netease' || source === 'kugou') {
    return source
  }
  if (activePanelView.value === 'netease' || activePanelView.value === 'kugou') {
    return activePanelView.value
  }
  if (immersivePlaylistSource.value === 'netease' || immersivePlaylistSource.value === 'kugou') {
    return immersivePlaylistSource.value
  }
  return 'general'
}

function onlinePlaybackQualityPlatformForTrack(track: MusicTrack | null): OnlinePlaybackQualityPlatform {
  if (track?.source === 'netease' || track?.source === 'kugou') {
    return track.source
  }
  return onlinePlaybackQualityPlatform.value
}

function mapOnlinePlaybackQualityToPlatform(
  value: OnlinePlaybackQuality,
  platform: OnlinePlaybackQualityPlatform,
) {
  if (platform === 'netease') {
    if (
      value === 'highest' ||
      value === 'jymaster' ||
      value === 'viper_clear' ||
      value === 'viper_hifi' ||
      value === 'viper_tape' ||
      value === 'viper_atmos' ||
      value === 'multitrack' ||
      value === 'super'
    ) {
      return 'highest'
    }
    if (value === 'flac') {
      return 'lossless'
    }
    if (value === 'high' || value === '320') {
      return 'exhigh'
    }
    if (value === '128') {
      return 'standard'
    }
  } else if (platform === 'kugou') {
    if (value === 'highest' || value === 'jymaster' || value === 'sky' || value === 'jyeffect') {
      return 'viper_clear'
    }
    if (value === 'lossless') {
      return 'flac'
    }
    if (value === 'exhigh' || value === '320') {
      return 'high'
    }
    if (value === '128') {
      return 'standard'
    }
  } else {
    if (
      value === 'jymaster' ||
      value === 'sky' ||
      value === 'jyeffect' ||
      value === 'hires' ||
      value === 'viper_clear' ||
      value === 'viper_hifi' ||
      value === 'viper_tape' ||
      value === 'viper_atmos' ||
      value === 'multitrack' ||
      value === 'super'
    ) {
      return 'highest'
    }
    if (value === 'flac') {
      return 'lossless'
    }
    if (value === 'exhigh' || value === '320') {
      return 'high'
    }
    if (value === '128') {
      return 'standard'
    }
  }
  return value
}

function normalizeOnlinePlaybackQualityForPlatform(
  value: string | OnlinePlaybackQuality | null | undefined,
  platform: OnlinePlaybackQualityPlatform,
) {
  const normalized = normalizeOnlinePlaybackQuality(value)
  const mapped = mapOnlinePlaybackQualityToPlatform(normalized, platform)
  const options = onlinePlaybackQualityOptionsForPlatform(platform)
  return options.some((option) => option.value === mapped) ? mapped : options[0].value
}

function onlinePlaybackQualityForPlatform(platform: OnlinePlaybackQualityPlatform) {
  if (platform === 'netease') {
    return normalizeOnlinePlaybackQualityForPlatform(neteaseOnlinePlaybackQuality.value, platform)
  }
  if (platform === 'kugou') {
    return normalizeOnlinePlaybackQualityForPlatform(kugouOnlinePlaybackQuality.value, platform)
  }
  return normalizeOnlinePlaybackQualityForPlatform(onlinePlaybackQuality.value, platform)
}

function setOnlinePlaybackQualityPreference(
  platform: OnlinePlaybackQualityPlatform,
  value: OnlinePlaybackQuality,
) {
  const nextQuality = normalizeOnlinePlaybackQualityForPlatform(value, platform)
  if (platform === 'netease') {
    neteaseOnlinePlaybackQuality.value = nextQuality
  } else if (platform === 'kugou') {
    kugouOnlinePlaybackQuality.value = nextQuality
  } else {
    onlinePlaybackQuality.value = nextQuality
  }
  onlinePlaybackQuality.value = normalizeOnlinePlaybackQualityForPlatform(nextQuality, 'general')
  return nextQuality
}

function onlinePlaybackQualityOption(value: OnlinePlaybackQuality, platform: OnlinePlaybackQualityPlatform) {
  const platformOptions = onlinePlaybackQualityOptionsForPlatform(platform)
  return (
    platformOptions.find((option) => option.value === value) ??
    ALL_ONLINE_PLAYBACK_QUALITY_OPTIONS.find((option) => option.value === value) ??
    platformOptions[0]
  )
}

function onlinePlaybackQualityOptionLabel(
  value: OnlinePlaybackQuality,
  platform: OnlinePlaybackQualityPlatform,
) {
  return onlinePlaybackQualityOption(value, platform).label
}

function toggleImmersiveQualityMenu() {
  if (onlinePlaybackQualitySwitching.value) {
    return
  }
  if (
    !immersiveQualityMenuOpen.value &&
    onlinePlaybackQualityPlatform.value === 'kugou' &&
    currentKugouQualityTrack.value
  ) {
    void ensureKugouQualityAvailability(currentKugouQualityTrack.value).catch(() => {})
  }
  immersiveQualityMenuOpen.value = !immersiveQualityMenuOpen.value
}

function closeImmersiveQualityMenuOnFocusOut(event: FocusEvent) {
  const current = event.currentTarget
  const next = event.relatedTarget
  if (current instanceof HTMLElement && next instanceof Node && current.contains(next)) {
    return
  }
  immersiveQualityMenuOpen.value = false
}

async function selectImmersiveOnlinePlaybackQuality(value: OnlinePlaybackQuality) {
  immersiveQualityMenuOpen.value = false
  await setOnlinePlaybackQuality(value)
}

async function setOnlinePlaybackQuality(value: OnlinePlaybackQuality) {
  if (onlinePlaybackQualitySwitching.value) {
    return
  }

  const platform = onlinePlaybackQualityPlatform.value
  if (!(await canApplyOnlinePlaybackQuality(value, platform))) {
    immersiveQualityMenuOpen.value = false
    return
  }

  const previousQuality = onlinePlaybackQualityForPlatform(platform)
  const nextQuality = setOnlinePlaybackQualityPreference(platform, value)
  immersiveQualityMenuOpen.value = false
  clearOnlinePlaybackRuntimeCache()
  await applyOnlinePlaybackQualityToCurrentTrack(nextQuality, platform, previousQuality)
}

async function canApplyOnlinePlaybackQuality(
  value: OnlinePlaybackQuality,
  platform: OnlinePlaybackQualityPlatform,
) {
  if (platform !== 'kugou') {
    return true
  }

  const nextQuality = normalizeOnlinePlaybackQualityForPlatform(value, platform)
  const track = currentKugouQualityTrack.value
  if (!track) {
    return true
  }

  try {
    const availability = await ensureKugouQualityAvailability(track)
    const unavailableReason = kugouQualityUnavailableReason(availability, nextQuality)
    if (!unavailableReason) {
      return true
    }

    const label = onlinePlaybackQualityOptionLabel(nextQuality, platform)
    playerStatus.value = ''
    playerError.value = `酷狗音质不可用：当前歌曲不能切换到「${label}」。${unavailableReason}`
    return false
  } catch (err) {
    if (nextQuality !== 'hires') {
      return true
    }

    playerStatus.value = ''
    playerError.value = `酷狗音质不可用：Hi-Res 预检失败，已停止切换。${normalizeOnlinePlaybackFailureReason(err)}`
    return false
  }
}

async function applyOnlinePlaybackQualityToCurrentTrack(
  nextQuality: OnlinePlaybackQuality,
  platform: OnlinePlaybackQualityPlatform,
  previousQuality: OnlinePlaybackQuality,
) {
  const expectedTrack = currentTrack.value
  const playbackPlatform = onlinePlaybackQualityPlatformForTrack(expectedTrack)
  const label = onlinePlaybackQualityOptionLabel(nextQuality, platform)
  const platformLabel = onlinePlaybackQualityPlatformText(playbackPlatform)
  if (
    !expectedTrack ||
    !currentTrackOnline.value ||
    !playing.value ||
    !audio.value ||
    !audio.value.src
  ) {
    playerError.value = ''
    playerStatus.value = currentTrackOnline.value
      ? `${platformLabel}已设为「${label}」，下次播放或重新播放当前歌曲时生效。`
      : `${platformLabel}已设为「${label}」。`
    return
  }

  const resumeTime = Math.max(audio.value.currentTime || currentTime.value || 0, 0)
  const previousAudioSource = audio.value.currentSrc || audio.value.src || expectedTrack.url
  onlinePlaybackQualitySwitching.value = true
  resetOnlineStallRecovery()
  playerError.value = ''
  playerStatus.value = `正在切换${platformLabel}到「${label}」，将从 ${formatTime(resumeTime)} 继续...`

  try {
    await refreshCurrentOnlinePlaybackAt(resumeTime, expectedTrack, { reason: 'quality-switch' })
  } catch (err) {
    if (sameMusicTrackIdentity(currentTrack.value, expectedTrack)) {
      const reason = normalizeOnlinePlaybackFailureReason(err)
      const previousLabel = onlinePlaybackQualityOptionLabel(previousQuality, platform)
      setOnlinePlaybackQualityPreference(platform, previousQuality)
      clearOnlinePlaybackRuntimeCache()
      const restored = await restoreCurrentOnlinePlaybackAfterQualitySwitch(
        expectedTrack,
        resumeTime,
        previousAudioSource,
      )
      playerStatus.value = restored
        ? `${platformLabel}「${label}」不可用，已恢复「${previousLabel}」并从 ${formatTime(resumeTime)} 继续播放。`
        : `已恢复${platformLabel}偏好为「${previousLabel}」，但当前播放链路需要重新播放。`
      playerError.value = `${platformLabel}切换失败：${reason}`
    }
  } finally {
    onlinePlaybackQualitySwitching.value = false
  }
}

async function restoreCurrentOnlinePlaybackAfterQualitySwitch(
  previousTrack: MusicTrack,
  resumeTime: number,
  previousAudioSource: string,
) {
  if (!sameMusicTrackIdentity(currentTrack.value, previousTrack)) {
    return false
  }

  neteaseCurrentTrack.value = previousTrack
  currentTime.value = resumeTime
  visualPlaybackTime.value = resumeTime
  playerError.value = ''
  await nextTick()

  if (!audio.value || !sameMusicTrackIdentity(currentTrack.value, previousTrack)) {
    return false
  }

  const resumed = await loadAndPlayCurrentAudioAt(
    previousTrack,
    resumeTime,
    previousAudioSource || previousTrack.url,
  )
  if (!resumed) {
    return false
  }

  try {
    await waitForOnlinePlaybackProgress(previousTrack, resumeTime, { requireAdvance: false })
  } catch {
    return playing.value && !playerError.value
  }
  return true
}

function normalizePlaybackLevel(value?: string | null) {
  return value?.trim() || null
}

function normalizePlaybackBitrate(value?: number | null) {
  return typeof value === 'number' && Number.isFinite(value) && value > 0
    ? Math.round(value)
    : null
}

function normalizePlaybackFileType(value?: string | null) {
  return value?.trim().toLowerCase() || null
}

function normalizePlaybackSize(value?: number | null) {
  return typeof value === 'number' && Number.isFinite(value) && value > 0
    ? Math.round(value)
    : null
}

function safeConvertFileSrc(path: string) {
  try {
    return convertFileSrc(path)
  } catch {
    return path
  }
}

function normalizeCoverImgUrl(value?: string | null) {
  const url = value?.trim() ?? ''
  if (!url) {
    return null
  }

  if (url.startsWith('data:image/') || /^https?:\/\//i.test(url)) {
    return url
  }

  return null
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

function mergeNeteaseSearchTracks(
  existingTracks: NeteasePlaylistTrack[],
  nextTracks: NeteasePlaylistTrack[],
) {
  const seenIds = new Set<number>()
  return [...existingTracks, ...nextTracks].filter((track) => {
    if (!Number.isFinite(track.id) || seenIds.has(track.id)) {
      return false
    }

    seenIds.add(track.id)
    return true
  })
}

function mergeKugouSearchTracks(
  existingTracks: KugouSearchTrack[],
  nextTracks: KugouSearchTrack[],
) {
  const seenKeys = new Set<string>()
  return [...existingTracks, ...nextTracks].filter((track) => {
    const key = track.hash.trim() || track.id.trim()
    if (!key || seenKeys.has(key)) {
      return false
    }

    seenKeys.add(key)
    return true
  })
}

function formatPlatformSearchNotice(
  platformLabel: string,
  keyword: string,
  loadedCount: number,
  total: number,
) {
  if (loadedCount === 0) {
    return `没有搜索到“${keyword}”。`
  }

  return total > 0
    ? `已加载 ${loadedCount} / ${total} 首${platformLabel}歌曲。`
    : `已加载 ${loadedCount} 首${platformLabel}歌曲。`
}

function formatPlatformPlaylistNotice(
  platformLabel: string,
  playlistName: string,
  loadedCount: number,
  total: number,
  hasMore: boolean,
) {
  const prefix = total > 0
    ? `已加载《${playlistName}》${loadedCount} / ${total} 首${platformLabel}歌单歌曲`
    : `已加载《${playlistName}》${loadedCount} 首${platformLabel}歌单歌曲`
  return hasMore ? `${prefix}，可继续加载。` : `${prefix}。`
}

function immersiveSearchPlatformName(platform: ImmersiveSearchPlatform) {
  return platform === 'netease' ? '网易云' : '酷狗'
}

function immersiveSearchPlatformSelected(value: ImmersiveSearchPlatformOptionValue) {
  if (value === 'all') {
    return activeImmersiveSearchPlatforms.value.length === IMMERSIVE_SEARCH_PLATFORMS.length
  }

  return immersiveSearchPlatforms.value.includes(value)
}

function toggleImmersiveSearchPlatform(value: ImmersiveSearchPlatformOptionValue) {
  immersiveSearchFocused.value = true
  let changed = false
  if (value === 'all') {
    changed =
      activeImmersiveSearchPlatforms.value.length !== IMMERSIVE_SEARCH_PLATFORMS.length
    immersiveSearchPlatforms.value = [...IMMERSIVE_SEARCH_PLATFORMS]
  } else {
    const current = activeImmersiveSearchPlatforms.value
    if (current.includes(value)) {
      if (current.length <= 1) {
        return
      }
      immersiveSearchPlatforms.value = current.filter((platform) => platform !== value)
      changed = true
    } else {
      immersiveSearchPlatforms.value = [...current, value]
      changed = true
    }
  }

  if (changed && immersiveSearchExecuted.value && immersiveSearchQuery.value.trim()) {
    void searchImmersiveSongs()
  }
}

function handleImmersiveSearchFocus() {
  immersiveSearchFocused.value = true
}

function handleImmersiveSearchInput() {
  immersiveSearchError.value = ''
  immersiveSearchNotice.value = ''
  if (immersiveSearchQuery.value.trim()) {
    return
  }

  immersiveSearchExecuted.value = false
  immersiveNeteaseSearchResult.value = null
  immersiveKugouSearchResult.value = null
}

function handleImmersiveSearchDocumentPointerDown(event: PointerEvent) {
  if (!immersiveMode.value || !immersiveSearchFocused.value) {
    return
  }

  const target = event.target
  if (target instanceof Node && immersiveSearchRoot.value?.contains(target)) {
    return
  }

  const activeElement = document.activeElement
  if (
    activeElement instanceof HTMLElement &&
    immersiveSearchRoot.value?.contains(activeElement)
  ) {
    activeElement.blur()
  }
  immersiveSearchInput.value?.blur()
  immersiveSearchFocused.value = false
}

function syncImmersiveSearchResultToPlatformState(
  keyword: string,
  result: NeteaseSearchResult | KugouSearchResult,
  platform: ImmersiveSearchPlatform,
) {
  if (platform === 'netease') {
    const neteaseResult = result as NeteaseSearchResult
    immersiveNeteaseSearchResult.value = neteaseResult
    neteaseSearchQuery.value = keyword
    neteaseSearchResult.value = neteaseResult
    neteaseSearchPage.value = 1
    neteaseSearchError.value = ''
    neteaseSearchNotice.value = formatPlatformSearchNotice(
      '网易云',
      neteaseResult.keyword,
      neteaseResult.tracks.length,
      neteaseResult.total,
    )
    return
  }

  const kugouResult = result as KugouSearchResult
  immersiveKugouSearchResult.value = kugouResult
  kugouSearchQuery.value = keyword
  kugouSearchResult.value = kugouResult
  kugouSearchPage.value = 1
  kugouSearchError.value = ''
  kugouSearchNotice.value = formatPlatformSearchNotice(
    '酷狗',
    kugouResult.keyword,
    kugouResult.tracks.length,
    kugouResult.total,
  )
}

async function searchImmersiveSongs() {
  immersiveSearchFocused.value = true
  const keyword = immersiveSearchQuery.value.trim()
  if (!keyword) {
    immersiveSearchError.value = '请输入搜索关键词。'
    immersiveSearchExecuted.value = true
    return
  }

  const platforms = activeImmersiveSearchPlatforms.value
  if (platforms.length === 0) {
    immersiveSearchError.value = '请至少选择一个音乐平台。'
    immersiveSearchExecuted.value = true
    return
  }

  const requestSerial = immersiveSearchRequestSerial + 1
  immersiveSearchRequestSerial = requestSerial
  immersiveSearchLoading.value = true
  immersiveSearchExecuted.value = true
  immersiveSearchError.value = ''
  immersiveSearchNotice.value = `正在搜索${immersiveSearchPlatformLabel.value}...`

  if (!platforms.includes('netease')) {
    immersiveNeteaseSearchResult.value = null
  } else {
    neteaseSearchLoading.value = true
    neteaseSearchError.value = ''
  }

  if (!platforms.includes('kugou')) {
    immersiveKugouSearchResult.value = null
  } else {
    kugouSearchLoading.value = true
    kugouSearchError.value = ''
  }

  const searchJobs = platforms.map(async (platform) => {
    if (platform === 'netease') {
      const result = await invoke<NeteaseSearchResult>('search_netease_songs', {
        keyword,
        page: 1,
        limit: IMMERSIVE_SEARCH_PAGE_SIZE,
      })
      return { platform, result }
    }

    const result = await invoke<KugouSearchResult>('search_kugou_songs', {
      keyword,
      page: 1,
      limit: IMMERSIVE_SEARCH_PAGE_SIZE,
    })
    return { platform, result }
  })

  const settledResults = await Promise.allSettled(searchJobs)
  if (requestSerial !== immersiveSearchRequestSerial) {
    return
  }

  const failures: string[] = []
  for (const settled of settledResults) {
    if (settled.status === 'fulfilled') {
      syncImmersiveSearchResultToPlatformState(
        keyword,
        settled.value.result,
        settled.value.platform,
      )
    } else {
      failures.push(String(settled.reason))
    }
  }

  const successCount = settledResults.length - failures.length
  if (successCount === 0) {
    immersiveSearchError.value = `沉浸搜索失败：${failures[0] ?? '平台没有返回结果。'}`
    immersiveSearchNotice.value = ''
  } else {
    const resultCount = immersiveSearchResults.value.length
    const suffix = failures.length > 0 ? `；${failures.length} 个平台失败` : ''
    immersiveSearchNotice.value =
      resultCount > 0
        ? `已从${immersiveSearchPlatformLabel.value}找到 ${resultCount} 首歌曲${suffix}。`
        : `没有搜索到“${keyword}”${suffix}。`
  }

  if (platforms.includes('netease')) {
    neteaseSearchLoading.value = false
  }
  if (platforms.includes('kugou')) {
    kugouSearchLoading.value = false
  }
  immersiveSearchLoading.value = false
}

function immersiveSearchResultPlatformLabel(item: ImmersiveSearchResultItem) {
  return item.platform === 'netease' ? '网易云' : '酷狗'
}

function immersiveSearchResultTitle(item: ImmersiveSearchResultItem) {
  return item.track.name
}

function immersiveSearchResultArtist(item: ImmersiveSearchResultItem) {
  return item.platform === 'netease'
    ? formatNeteaseTrackArtists(item.track)
    : formatKugouTrackArtists(item.track)
}

function immersiveSearchResultAlbum(item: ImmersiveSearchResultItem) {
  return normalizeTrackAlbum(item.track.album)
}

function immersiveSearchResultDurationLabel(item: ImmersiveSearchResultItem) {
  return item.platform === 'netease'
    ? immersiveNeteaseTrackDurationLabel(item.track)
    : immersiveKugouTrackDurationLabel(item.track)
}

function immersiveSearchResultUnavailableReason(item: ImmersiveSearchResultItem) {
  return item.platform === 'netease'
    ? neteaseTrackUnavailableReason(item.track)
    : kugouTrackUnavailableReason(item.track)
}

function immersiveSearchResultActive(item: ImmersiveSearchResultItem) {
  if (item.platform === 'netease') {
    return immersiveNeteaseTrackActive(item.track)
  }

  return immersiveKugouTrackActive(item.track)
}

function canPlayImmersiveSearchResult(item: ImmersiveSearchResultItem) {
  if (item.platform === 'netease') {
    return canPlayImmersiveNeteaseTrack(item.track)
  }

  return canPlayImmersiveKugouTrack(item.track)
}

async function playImmersiveSearchResult(item: ImmersiveSearchResultItem) {
  if (item.platform === 'netease') {
    immersivePlaylistSource.value = 'netease'
    await playNeteaseTrack(item.track, immersiveNeteaseSearchTracks.value)
    return
  }

  immersivePlaylistSource.value = 'kugou'
  await playKugouTrack(item.track, immersiveKugouSearchTracks.value)
}

async function searchNeteaseSongs(loadMore = false) {
  const keyword = loadMore
    ? (neteaseSearchResult.value?.keyword ?? neteaseSearchQuery.value).trim()
    : neteaseSearchQuery.value.trim()
  if (!keyword) {
    neteaseSearchError.value = '请输入网易云搜索关键词。'
    return
  }

  neteaseSearchLoading.value = true
  neteaseSearchError.value = ''
  const nextPage = loadMore ? neteaseSearchPage.value + 1 : 1
  neteaseSearchNotice.value = loadMore
    ? `正在加载“${keyword}”的更多网易云结果...`
    : `正在搜索“${keyword}”...`

  try {
    const result = await invoke<NeteaseSearchResult>('search_netease_songs', {
      keyword,
      page: nextPage,
      limit: PLATFORM_SEARCH_PAGE_SIZE,
    })
    if (loadMore && neteaseSearchResult.value?.keyword === result.keyword) {
      const mergedTracks = mergeNeteaseSearchTracks(
        neteaseSearchResult.value.tracks,
        result.tracks,
      )
      neteaseSearchResult.value = {
        ...result,
        tracks: mergedTracks,
        total: result.total || neteaseSearchResult.value.total,
      }
    } else {
      neteaseSearchResult.value = result
    }
    neteaseSearchPage.value = nextPage
    neteaseSearchNotice.value = formatPlatformSearchNotice(
      '网易云',
      result.keyword,
      neteaseSearchResult.value.tracks.length,
      neteaseSearchResult.value.total,
    )
    if (immersiveMode.value) {
      immersivePlaylistSource.value = 'netease'
    }
  } catch (err) {
    if (!loadMore) {
      neteaseSearchResult.value = null
      neteaseSearchPage.value = 0
    }
    neteaseSearchError.value = `网易云搜索失败：${String(err)}`
  } finally {
    neteaseSearchLoading.value = false
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
    neteasePlaylistDetailPage.value = 0
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

async function loadNeteasePlaylistDetail(
  playlist: NeteasePlaylistSummary,
  showStatus = true,
  loadMore = false,
) {
  const isSamePlaylist = neteasePlaylistDetail.value?.playlist.id === playlist.id
  const nextPage = loadMore && isSamePlaylist ? neteasePlaylistDetailPage.value + 1 : 1
  neteaseSelectedPlaylistId.value = playlist.id
  neteasePlaylistDetailLoading.value = true
  neteasePlaylistError.value = ''
  if (!loadMore || !isSamePlaylist) {
    neteasePlaylistDetail.value = null
    neteasePlaylistDetailPage.value = 0
  }
  if (showStatus) {
    neteaseLoginNotice.value = loadMore
      ? `正在加载《${playlist.name}》的更多歌曲...`
      : `正在读取《${playlist.name}》...`
  }

  try {
    const result = await invoke<NeteasePlaylistDetail>(
      'get_netease_playlist_detail',
      {
        playlistId: playlist.id,
        page: nextPage,
        limit: PLATFORM_PLAYLIST_PAGE_SIZE,
      },
    )
    if (loadMore && isSamePlaylist && neteasePlaylistDetail.value) {
      const mergedTracks = mergeNeteaseSearchTracks(
        neteasePlaylistDetail.value.tracks,
        result.tracks,
      )
      neteasePlaylistDetail.value = {
        ...result,
        tracks: mergedTracks,
        totalTrackCount: result.totalTrackCount || neteasePlaylistDetail.value.totalTrackCount,
        message: formatPlatformPlaylistNotice(
          '网易云',
          result.playlist.name,
          mergedTracks.length,
          result.totalTrackCount || neteasePlaylistDetail.value.totalTrackCount,
          result.truncated,
        ),
      }
    } else {
      neteasePlaylistDetail.value = {
        ...result,
        message: formatPlatformPlaylistNotice(
          '网易云',
          result.playlist.name,
          result.tracks.length,
          result.totalTrackCount,
          result.truncated,
        ),
      }
    }
    neteasePlaylistDetailPage.value = nextPage
    if (showStatus) {
      neteaseLoginNotice.value = neteasePlaylistDetail.value.message
    }
  } catch (err) {
    if (!loadMore) {
      neteasePlaylistDetail.value = null
      neteasePlaylistDetailPage.value = 0
    }
    neteasePlaylistError.value = `网易云歌单歌曲读取失败：${String(err)}`
  } finally {
    neteasePlaylistDetailLoading.value = false
  }
}

async function loadMoreNeteasePlaylistTracks() {
  const playlist = neteaseSelectedPlaylist.value
  if (!playlist || !neteasePlaylistHasMore.value || neteasePlaylistDetailLoading.value) {
    return
  }

  await loadNeteasePlaylistDetail(playlist, true, true)
}

async function openNeteaseLoginFromLeft() {
  activePanelView.value = 'netease'
  await startNeteaseQrLogin()
}

async function refreshNeteasePlaylistsFromLeft() {
  activePanelView.value = 'netease'
  await refreshNeteasePlaylists(true)
}

function scrollElementIntoView(element: HTMLElement | null) {
  if (!element) {
    return
  }

  const prefersReducedMotion =
    typeof window.matchMedia === 'function'
      ? window.matchMedia('(prefers-reduced-motion: reduce)').matches
      : false

  element.scrollIntoView({
    behavior: prefersReducedMotion ? 'auto' : 'smooth',
    block: 'start',
    inline: 'nearest',
  })
}

async function scrollPlaylistDetailIntoView(source: 'netease' | 'kugou') {
  await nextTick()
  await new Promise<void>((resolve) => {
    window.requestAnimationFrame(() => resolve())
  })

  scrollElementIntoView(
    source === 'netease'
      ? neteasePlaylistDetailSection.value
      : kugouPlaylistDetailSection.value,
  )
}

async function openNeteasePlaylistFromLeft(playlist: NeteasePlaylistSummary) {
  activePanelView.value = 'netease'
  await loadNeteasePlaylistDetail(playlist)
  await scrollPlaylistDetailIntoView('netease')
}

function readOnlinePlaybackCache<T, K>(
  cache: Map<K, OnlinePlaybackCacheEntry<T>>,
  key: K,
) {
  const entry = cache.get(key)
  if (!entry) {
    return null
  }

  if (entry.expiresAt <= Date.now()) {
    cache.delete(key)
    return null
  }

  return entry.playback
}

function writeOnlinePlaybackCache<T, K>(
  cache: Map<K, OnlinePlaybackCacheEntry<T>>,
  key: K,
  playback: T,
  ttlMs = ONLINE_PLAYBACK_CACHE_TTL_MS,
) {
  cache.set(key, {
    playback,
    expiresAt: Date.now() + ttlMs,
  })
}

function normalizeOnlinePlaybackFailureReason(err: unknown) {
  const message = String(err)
    .replace(/^Error:\s*/i, '')
    .replace(/^InvokeError:\s*/i, '')
    .replace(/^网易云在线播放失败：/, '')
    .replace(/^酷狗在线播放失败：/, '')
    .trim()
  return message || '平台没有返回可播放链接。'
}

function stripPlaybackFailurePrefix(value: string) {
  return value
    .replace(/^Error:\s*/i, '')
    .replace(/^InvokeError:\s*/i, '')
    .replace(/^网易云在线播放失败：/, '')
    .replace(/^酷狗在线播放失败：/, '')
    .replace(/^网易云切换失败：/, '')
    .replace(/^酷狗切换失败：/, '')
    .trim()
}

function truncatePlaybackFailureText(value: string, maxLength = 96) {
  const text = value.replace(/\s+/g, ' ').trim()
  if (text.length <= maxLength) {
    return text
  }
  return `${text.slice(0, maxLength).trim()}...`
}

function formatPlaybackFailureDisplay(value: string): PlaybackFailureDisplay {
  const raw = stripPlaybackFailurePrefix(value)
  const fallbackSummary = truncatePlaybackFailureText(raw || value || '平台没有返回可播放链接。', 128)
  const display: PlaybackFailureDisplay = {
    title: '播放失败',
    summary: fallbackSummary,
    compact: fallbackSummary,
    detail: shouldShowPlaybackFailureDetail(raw, fallbackSummary) ? raw : '',
    hints: [],
  }

  if (!raw) {
    display.summary = '平台没有返回具体失败原因。'
    display.compact = display.summary
    return display
  }

  if (raw.includes('当前酷狗在线音乐无法继续读取')) {
    display.title = '酷狗代理读取中断'
    display.summary = raw.includes('播放代理预检通过')
      ? '播放地址预检可读，但实际播放到后段时 Range 续读失败。'
      : '本机播放代理没有继续读到后续音频数据。'
    display.compact = display.title
    display.hints = [
      '通常与 CDN 临时波动、链接过期或拖动进度后的 Range 请求中断有关。',
      '可以重试当前歌曲，或切换音质重新获取播放链路。',
    ]
    display.detail = shouldShowPlaybackFailureDetail(raw, display.summary) ? raw : ''
    return display
  }

  if (raw.includes('酷狗 Hi-Res 播放源获取失败')) {
    display.title = '酷狗 Hi-Res 当前不可用'
    display.summary = raw.includes('auth_through=空数组') || raw.includes('priv_status=0')
      ? '登录态没有拿到 Hi-Res 授权，接口未返回 Hi-Res 播放源。'
      : '登录态 /v5/url 没有返回 Hi-Res 播放源。'
    display.compact = display.title
    display.hints = [
      '切换到无损、高品或标准音质可继续尝试播放。',
      '官方客户端可播不代表当前接口能拿到同等设备或风控授权。',
    ]
    return display
  }

  if (
    raw.includes('没有拿到完整播放授权') ||
    raw.includes('需要会员/付费授权') ||
    (raw.includes('需要付费') && raw.includes('试听片段'))
  ) {
    display.title = '平台未返回完整播放授权'
    display.summary = '当前账号在项目内没有拿到完整音频授权，已停止使用试听片段。'
    display.compact = display.title
    display.hints = ['可切换较低音质、重新登录平台账号，或等待后续客户端/网页播放兜底。']
    return display
  }

  if (raw.includes('该音质不可用') || raw.includes('不满足所选')) {
    display.title = '所选音质当前不可用'
    display.summary = '平台返回的播放源不满足当前选择的音质，已拒绝把低音质当作切换成功。'
    display.compact = display.title
    display.hints = ['可改选平台实际返回的较低音质后重试。']
    return display
  }

  if (raw.includes('播放链接为空') || raw.includes('没有返回播放链接') || raw.includes('未返回可访问音频链接')) {
    display.title = '平台没有返回播放源'
    display.summary = '接口没有返回可播放音频链接，可能与版权、会员、地区、设备授权或当前音质有关。'
    display.compact = display.title
    return display
  }

  if (raw.includes('版权')) {
    display.title = '当前歌曲版权受限'
    display.summary = '平台没有向项目返回这首歌的可播放链接。'
    display.compact = display.title
    return display
  }

  if (raw.includes('会员') || raw.includes('付费')) {
    display.title = '当前歌曲需要平台授权'
    display.summary = '平台没有向当前接口返回完整播放源，可能需要会员包、付费授权或官方客户端授权。'
    display.compact = display.title
    return display
  }

  return display
}

function shouldShowPlaybackFailureDetail(raw: string, summary: string) {
  if (!raw.trim()) {
    return false
  }

  const normalizedRaw = raw.replace(/\s+/g, ' ').trim()
  const normalizedSummary = summary.replace(/\s+/g, ' ').trim()
  if (normalizedRaw === normalizedSummary || normalizedRaw.length <= normalizedSummary.length + 12) {
    return false
  }

  return [
    'status=',
    'error_code=',
    'auth_through',
    'priv_status',
    'trans_param',
    'fail_process',
    'tracker_through',
    'data字段=',
    '失败位置：',
    '已刷新',
  ].some((keyword) => normalizedRaw.includes(keyword))
}

function compactPlaybackFailureReason(value: string) {
  return formatPlaybackFailureDisplay(value).compact
}

function playbackFailureDetailTitle(value: string) {
  const display = formatPlaybackFailureDisplay(value)
  return display.detail
    ? `${display.title}：${display.summary}\n\n${display.detail}`
    : `${display.title}：${display.summary}`
}

function handlePlayerErrorDetailToggle(event: Event) {
  if (event.target instanceof HTMLDetailsElement) {
    playerErrorDetailOpen.value = event.target.open
  }
}

function normalizeOnlineStallRecoveryFailureReason(err: unknown) {
  return normalizeOnlinePlaybackFailureReason(err)
    .replace(/https?:\/\/\S+/gi, '播放链接')
    .slice(0, 180)
    .trim() || '长时间无数据，刷新播放链路后仍未恢复。'
}

function isUnavailableOnlinePlaybackFailure(err: unknown) {
  const message = normalizeOnlinePlaybackFailureReason(err)
  return [
    '暂不能播放',
    '暂无版权',
    '版权',
    '资源不可用',
    '需要会员',
    '会员',
    '地区',
    '付费',
    '播放链接为空',
    '没有返回播放链接',
    '未返回可访问音频链接',
    '没有拿到完整播放授权',
    '只拿到试听片段',
    '试听片段',
    '该音质不可用',
    '设备注册未通过',
  ].some((keyword) => message.includes(keyword))
}

function unavailableTrackEntry<K>(cache: Map<K, OnlineUnavailableTrack>, key: K) {
  const entry = cache.get(key)
  if (!entry) {
    return null
  }

  if (entry.retryAfter <= Date.now()) {
    cache.delete(key)
    return null
  }

  return entry
}

function markOnlineTrackUnavailable<K>(
  cache: Map<K, OnlineUnavailableTrack>,
  key: K,
  reason: string,
) {
  cache.set(key, {
    reason,
    failedAt: Date.now(),
    retryAfter: Date.now() + ONLINE_UNAVAILABLE_RETRY_AFTER_MS,
  })
}

function neteaseTrackUnavailableReason(track: NeteasePlaylistTrack) {
  return unavailableTrackEntry(neteaseUnavailableTracks, track.id)?.reason ?? ''
}

function kugouTrackUnavailableReason(track: KugouSearchTrack) {
  const key = kugouPlaybackCacheKey(track)
  return key ? unavailableTrackEntry(kugouUnavailableTracks, key)?.reason ?? '' : ''
}

function clearNeteaseTrackUnavailable(track: NeteasePlaylistTrack) {
  neteaseUnavailableTracks.delete(track.id)
}

function clearKugouTrackUnavailable(track: KugouSearchTrack) {
  const key = kugouPlaybackCacheKey(track)
  if (key) {
    kugouUnavailableTracks.delete(key)
  }
}

function kugouQualityAvailabilityCacheKey(track: KugouSearchTrack) {
  const key = kugouPlaybackCacheKey(track)
  return key ? `${key}:${track.albumAudioId ?? 0}:${track.audioId ?? 0}` : ''
}

function getCachedKugouQualityAvailability(track: KugouSearchTrack) {
  const key = kugouQualityAvailabilityCacheKey(track)
  return key ? readOnlinePlaybackCache(kugouQualityAvailabilityCache, key) : null
}

function cacheKugouQualityAvailability(track: KugouSearchTrack, availability: KugouQualityAvailability) {
  const key = kugouQualityAvailabilityCacheKey(track)
  if (key) {
    writeOnlinePlaybackCache(
      kugouQualityAvailabilityCache,
      key,
      availability,
      KUGOU_QUALITY_AVAILABILITY_CACHE_TTL_MS,
    )
  }
}

function clearKugouQualityAvailabilityRuntimeCache() {
  kugouQualityAvailabilityCache.clear()
  kugouQualityAvailabilityInflight.clear()
  kugouQualityAvailabilityLoading.value = false
  kugouQualityAvailabilityError.value = ''
}

function fetchKugouQualityAvailability(track: KugouSearchTrack) {
  const key = kugouQualityAvailabilityCacheKey(track)
  if (!key) {
    return Promise.reject(new Error('酷狗歌曲缺少 hash，无法预检音质。'))
  }

  const existing = kugouQualityAvailabilityInflight.get(key)
  if (existing) {
    return existing
  }

  const request = invoke<KugouQualityAvailability>('get_kugou_song_quality_availability', {
    hash: track.hash,
    hashCandidates: track.hashCandidates ?? [track.hash],
    albumAudioId: track.albumAudioId ?? null,
  }).finally(() => {
    kugouQualityAvailabilityInflight.delete(key)
  })
  kugouQualityAvailabilityInflight.set(key, request)
  return request
}

async function ensureKugouQualityAvailability(track: KugouSearchTrack) {
  const cached = getCachedKugouQualityAvailability(track)
  if (cached) {
    return cached
  }

  kugouQualityAvailabilityLoading.value = true
  kugouQualityAvailabilityError.value = ''
  try {
    const availability = await fetchKugouQualityAvailability(track)
    cacheKugouQualityAvailability(track, availability)
    return availability
  } catch (err) {
    const reason = normalizeOnlinePlaybackFailureReason(err)
    kugouQualityAvailabilityError.value = reason
    throw err
  } finally {
    kugouQualityAvailabilityLoading.value = false
  }
}

function kugouQualityAvailabilityItem(
  availability: KugouQualityAvailability | null,
  quality: OnlinePlaybackQuality,
) {
  return availability?.qualities.find((item) => item.quality === quality) ?? null
}

function kugouQualityUnavailableReason(
  availability: KugouQualityAvailability | null,
  quality: OnlinePlaybackQuality,
) {
  const item = kugouQualityAvailabilityItem(availability, quality)
  return item?.status === 'unavailable'
    ? item.reason?.trim() || `${onlinePlaybackQualityOptionLabel(quality, 'kugou')}当前不可用。`
    : ''
}

function firstPlayableKugouQuality(availability: KugouQualityAvailability | null) {
  const options = KUGOU_ONLINE_PLAYBACK_QUALITY_OPTIONS.filter((option) => option.value !== 'hires')
  const available = options.find((option) =>
    kugouQualityAvailabilityItem(availability, option.value)?.status === 'available',
  )
  if (available) {
    return available.value
  }

  const notUnavailable = options.find((option) =>
    kugouQualityAvailabilityItem(availability, option.value)?.status !== 'unavailable',
  )
  return notUnavailable?.value ?? 'standard'
}

async function applyKugouQualityAvailabilityBeforePlayback(track: KugouSearchTrack) {
  const desiredQuality = onlinePlaybackQualityForPlatform('kugou')
  let availability: KugouQualityAvailability | null = null
  try {
    availability = await ensureKugouQualityAvailability(track)
  } catch {
    if (desiredQuality === 'hires') {
      const fallback: OnlinePlaybackQuality = 'flac'
      setOnlinePlaybackQualityPreference('kugou', fallback)
      playerStatus.value = `酷狗 Hi-Res 暂不能完成预检，已先改用「${onlinePlaybackQualityOptionLabel(fallback, 'kugou')}」。`
    }
    return
  }

  const unavailableReason = kugouQualityUnavailableReason(availability, desiredQuality)
  if (!unavailableReason) {
    return
  }

  const fallback = firstPlayableKugouQuality(availability)
  const fallbackLabel = onlinePlaybackQualityOptionLabel(fallback, 'kugou')
  setOnlinePlaybackQualityPreference('kugou', fallback)
  playerStatus.value = `所选酷狗音质「${onlinePlaybackQualityOptionLabel(
    desiredQuality,
    'kugou',
  )}」当前不可用，已改用「${fallbackLabel}」。${unavailableReason}`
}

function neteasePlaybackCacheKey(track: NeteasePlaylistTrack) {
  return `${track.id}:${onlinePlaybackQualityForPlatform('netease')}`
}

function getCachedNeteasePlayback(track: NeteasePlaylistTrack) {
  return readOnlinePlaybackCache(neteasePlaybackCache, neteasePlaybackCacheKey(track))
}

function cacheNeteasePlayback(track: NeteasePlaylistTrack, playback: NeteasePlaybackUrl) {
  writeOnlinePlaybackCache(neteasePlaybackCache, neteasePlaybackCacheKey(track), playback)
}

function fetchNeteasePlayback(track: NeteasePlaylistTrack) {
  const quality = onlinePlaybackQualityForPlatform('netease')
  const key = `${track.id}:${quality}`
  const existing = neteasePlaybackInflight.get(key)
  if (existing) {
    return existing
  }

  const request = invoke<NeteasePlaybackUrl>('get_netease_song_playback_url', {
    songId: track.id,
    level: quality,
  }).finally(() => {
    neteasePlaybackInflight.delete(key)
  })
  neteasePlaybackInflight.set(key, request)
  return request
}

async function resolveNeteaseTrackPlayback(track: NeteasePlaylistTrack) {
  const cached = getCachedNeteasePlayback(track)
  if (cached) {
    return { playback: cached, cached: true }
  }

  const playback = await fetchNeteasePlayback(track)
  cacheNeteasePlayback(track, playback)
  return { playback, cached: false }
}

function kugouPlaybackCacheKey(track: KugouSearchTrack) {
  return track.hash.trim()
}

function kugouPlaybackQualityCacheKey(track: KugouSearchTrack) {
  const key = kugouPlaybackCacheKey(track)
  return key ? `${key}:${onlinePlaybackQualityForPlatform('kugou')}` : ''
}

function getCachedKugouPlayback(track: KugouSearchTrack) {
  const key = kugouPlaybackQualityCacheKey(track)
  return key ? readOnlinePlaybackCache(kugouPlaybackCache, key) : null
}

function cacheKugouPlayback(track: KugouSearchTrack, playback: KugouPlaybackUrl) {
  const key = kugouPlaybackQualityCacheKey(track)
  if (key) {
    writeOnlinePlaybackCache(kugouPlaybackCache, key, playback)
  }
}

function fetchKugouPlayback(track: KugouSearchTrack) {
  const hashKey = kugouPlaybackCacheKey(track)
  if (!hashKey) {
    return Promise.reject(new Error('酷狗歌曲缺少 hash，无法获取播放地址。'))
  }

  const quality = onlinePlaybackQualityForPlatform('kugou')
  const key = `${hashKey}:${quality}`
  const existing = kugouPlaybackInflight.get(key)
  if (existing) {
    return existing
  }

  const request = invoke<KugouPlaybackUrl>('get_kugou_song_playback_url', {
    hash: track.hash,
    hashCandidates: track.hashCandidates ?? [track.hash],
    albumAudioId: track.albumAudioId ?? null,
    audioId: track.audioId ?? null,
    playbackQuality: quality,
  }).finally(() => {
    kugouPlaybackInflight.delete(key)
  })
  kugouPlaybackInflight.set(key, request)
  return request
}

async function resolveKugouTrackPlayback(track: KugouSearchTrack) {
  const cached = getCachedKugouPlayback(track)
  if (cached) {
    return { playback: cached, cached: true }
  }

  const playback = await fetchKugouPlayback(track)
  cacheKugouPlayback(track, playback)
  return { playback, cached: false }
}

function clearOnlinePlaybackRuntimeCache() {
  clearOnlinePlaybackPrefetchTimer()
  neteasePlaybackCache.clear()
  kugouPlaybackCache.clear()
  neteasePlaybackInflight.clear()
  kugouPlaybackInflight.clear()
}

function invalidateCurrentOnlinePlaybackCache() {
  const track = currentTrack.value
  if (track?.source === 'netease' && track.neteaseSongId) {
    const prefix = `${track.neteaseSongId}:`
    for (const key of Array.from(neteasePlaybackCache.keys())) {
      if (key.startsWith(prefix)) {
        neteasePlaybackCache.delete(key)
      }
    }
    for (const key of Array.from(neteasePlaybackInflight.keys())) {
      if (key.startsWith(prefix)) {
        neteasePlaybackInflight.delete(key)
      }
    }
    return
  }

  if (track?.source === 'kugou' && track.kugouSongHash) {
    const prefix = `${track.kugouSongHash}:`
    for (const key of Array.from(kugouPlaybackCache.keys())) {
      if (key.startsWith(prefix)) {
        kugouPlaybackCache.delete(key)
      }
    }
    for (const key of Array.from(kugouPlaybackInflight.keys())) {
      if (key.startsWith(prefix)) {
        kugouPlaybackInflight.delete(key)
      }
    }
  }
}

async function playNeteaseTrack(
  track: NeteasePlaylistTrack,
  contextTracks?: NeteasePlaylistTrack[],
  options: OnlinePlaybackOptions = {},
) {
  if (!neteaseLoggedIn.value) {
    neteasePlaylistError.value = '请先登录网易云音乐。'
    return
  }

  const sourceTracks = resolveNeteasePlaybackTracks(track, contextTracks)
  neteaseTrackActionId.value = track.id
  neteasePlaylistError.value = ''
  playerError.value = ''
  playerStatus.value = `正在获取《${track.name}》的在线播放地址...`

  try {
    const { playback, cached } = await resolveNeteaseTrackPlayback(track)
    clearNeteaseTrackUnavailable(track)
    const onlineTrack = createNeteaseMusicTrack(track, playback)
    pausePlayback()
    currentIndex.value = -1
    neteaseCurrentTrack.value = onlineTrack
    playbackContext.value = { source: 'netease', tracks: sourceTracks }
    if (immersiveMode.value) {
      immersivePlaylistSource.value = 'netease'
    }
    currentTime.value = 0
    visualPlaybackTime.value = 0
    duration.value = onlineTrack.duration ?? 0
    await nextTick()
    await playCurrent(true)
    if (!playerError.value) {
      playerStatus.value = cached
        ? `正在在线播放网易云《${track.name}》，已复用预取播放地址。`
        : `正在在线播放网易云《${track.name}》。`
      void showNeteaseLyrics(track, false)
      scheduleOnlinePlaybackPrefetch()
    }
  } catch (err) {
    const reason = normalizeOnlinePlaybackFailureReason(err)
    if (isUnavailableOnlinePlaybackFailure(err)) {
      markOnlineTrackUnavailable(neteaseUnavailableTracks, track.id, reason)
      if (options.autoSkip && await skipUnavailableNeteaseTrack(track, sourceTracks, reason)) {
        return
      }
    }
    playerStatus.value = ''
    playerError.value = `网易云在线播放失败：${reason}`
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
  neteasePlaylistDetailPage.value = 0
  neteasePlaylistsLoading.value = false
  neteasePlaylistDetailLoading.value = false
  neteasePlaylistError.value = ''
  closeNeteaseLyrics()
}

function resetNeteasePlaybackState() {
  if (neteaseCurrentTrack.value?.source === 'netease') {
    pausePlayback()
    neteaseCurrentTrack.value = null
    currentTime.value = 0
    visualPlaybackTime.value = 0
    duration.value = 0
  }
}

async function refreshKugouLoginStatus(showStatus = true) {
  kugouLoginError.value = ''
  if (showStatus) {
    kugouLoginNotice.value = '正在检查酷狗登录状态...'
  }

  try {
    const status = await invoke<KugouLoginStatus>('get_kugou_login_status')
    kugouLoginStatus.value = status
    clearKugouQualityAvailabilityRuntimeCache()
    if (!status.loggedIn) {
      resetKugouPlaylistState()
    }
    if (showStatus) {
      kugouLoginNotice.value = status.message
    }
  } catch (err) {
    kugouLoginError.value = `酷狗状态读取失败：${String(err)}`
  }
}

async function startKugouQrLogin() {
  stopKugouQrPolling()
  kugouLoginBusy.value = true
  kugouLoginError.value = ''
  kugouLoginNotice.value = '正在生成酷狗登录二维码...'
  kugouQrStatus.value = 'idle'

  try {
    kugouQrLogin.value = await invoke<KugouQrLogin>('create_kugou_qr_login')
    kugouQrStatus.value = 'waiting'
    kugouLoginNotice.value = '等待酷狗音乐 App 扫码。'
    startKugouQrPolling()
  } catch (err) {
    kugouQrLogin.value = null
    kugouQrStatus.value = 'error'
    kugouLoginError.value = `酷狗二维码生成失败：${String(err)}`
  } finally {
    kugouLoginBusy.value = false
  }
}

function cancelKugouQrLogin() {
  stopKugouQrPolling()
  kugouQrLogin.value = null
  kugouQrStatus.value = 'idle'
  kugouLoginNotice.value = kugouLoginStatus.value?.message ?? '已取消酷狗扫码登录。'
}

function startKugouQrPolling() {
  stopKugouQrPolling()
  void checkKugouQrLogin()
  kugouQrPollTimer = window.setInterval(() => {
    void checkKugouQrLogin()
  }, 2000)
}

function stopKugouQrPolling() {
  if (kugouQrPollTimer !== null) {
    window.clearInterval(kugouQrPollTimer)
    kugouQrPollTimer = null
  }
}

async function checkKugouQrLogin() {
  if (!kugouQrLogin.value || kugouQrChecking.value) {
    return
  }

  if (kugouQrExpired.value) {
    stopKugouQrPolling()
    kugouQrStatus.value = 'expired'
    kugouLoginNotice.value = '二维码已过期，请重新生成。'
    return
  }

  kugouQrChecking.value = true
  kugouLoginError.value = ''

  try {
    const result = await invoke<KugouQrCheckResult>('check_kugou_qr_login', {
      key: kugouQrLogin.value.key,
    })
    kugouQrStatus.value = normalizeKugouQrStatus(result)
    kugouLoginNotice.value = result.message

    if (result.loggedIn) {
      stopKugouQrPolling()
      kugouQrLogin.value = null
      kugouQrStatus.value = 'authorized'
      kugouLoginStatus.value = {
        loggedIn: true,
        profile: result.profile,
        savedAt: null,
        checkedAt: String(Math.floor(Date.now() / 1000)),
        message: result.message,
      }
      await refreshKugouLoginStatus(false)
      clearKugouQualityAvailabilityRuntimeCache()
      await refreshKugouPlaylists(false)
      playerStatus.value = '酷狗音乐登录成功。'
    } else if (kugouQrStatus.value === 'expired') {
      stopKugouQrPolling()
    }
  } catch (err) {
    stopKugouQrPolling()
    kugouQrStatus.value = 'error'
    kugouLoginError.value = `酷狗扫码状态检查失败：${String(err)}`
  } finally {
    kugouQrChecking.value = false
  }
}

async function clearKugouLogin() {
  if (kugouLoggedIn.value && !window.confirm('清除本机保存的酷狗登录状态？')) {
    return
  }

  stopKugouQrPolling()
  kugouLoginBusy.value = true
  kugouLoginError.value = ''
  kugouLoginNotice.value = '正在清除酷狗登录状态...'

  try {
    const status = await invoke<KugouLoginStatus>('clear_kugou_login')
    kugouLoginStatus.value = status
    kugouQrLogin.value = null
    kugouQrStatus.value = 'idle'
    kugouLoginNotice.value = status.message
    clearKugouQualityAvailabilityRuntimeCache()
    resetKugouPlaylistState()
  } catch (err) {
    kugouLoginError.value = `清除酷狗登录状态失败：${String(err)}`
  } finally {
    kugouLoginBusy.value = false
  }
}

async function refreshKugouPlaylists(showStatus = true) {
  if (!kugouLoggedIn.value) {
    kugouPlaylistError.value = '请先登录酷狗音乐。'
    return
  }

  kugouPlaylistsLoading.value = true
  kugouPlaylistError.value = ''
  if (showStatus) {
    kugouLoginNotice.value = '正在读取酷狗个人歌单...'
  }

  try {
    const playlists = await invoke<KugouPlaylistSummary[]>('list_kugou_playlists')
    kugouPlaylists.value = playlists
    kugouPlaylistDetail.value = null
    kugouPlaylistDetailPage.value = 0
    kugouSelectedPlaylistId.value = playlists[0]?.listId ?? ''
    kugouSelectedContentSource.value = playlists[0] ? 'personal' : kugouSelectedContentSource.value
    if (playlists[0]) {
      await loadKugouPlaylistDetail(playlists[0], false, false, 'personal')
    }
    if (showStatus) {
      kugouLoginNotice.value =
        playlists.length > 0 ? `已读取 ${playlists.length} 个酷狗歌单。` : '没有读取到酷狗歌单。'
    }
  } catch (err) {
    kugouPlaylistError.value = `酷狗歌单读取失败：${String(err)}`
  } finally {
    kugouPlaylistsLoading.value = false
  }
}

async function loadKugouPlaylistDetail(
  playlist: KugouPlaylistSummary,
  showStatus = true,
  loadMore = false,
  source: Exclude<KugouContentSource, ''> = 'personal',
) {
  const playlistKey = kugouPlaylistKey(playlist)
  const currentDetailKey = kugouPlaylistDetail.value
    ? kugouPlaylistKey(kugouPlaylistDetail.value.playlist)
    : ''
  const isSamePlaylist =
    kugouSelectedContentSource.value === source && currentDetailKey === playlistKey
  const nextPage = loadMore && isSamePlaylist ? kugouPlaylistDetailPage.value + 1 : 1
  kugouSelectedContentSource.value = source
  if (source === 'recommended') {
    kugouSelectedRecommendedPlaylistId.value = playlistKey
  } else {
    kugouSelectedPlaylistId.value = playlist.listId
  }
  kugouPlaylistDetailLoading.value = true
  kugouPlaylistError.value = ''
  if (!loadMore || !isSamePlaylist) {
    kugouPlaylistDetail.value = null
    kugouPlaylistDetailPage.value = 0
  }
  if (showStatus) {
    kugouLoginNotice.value = loadMore
      ? `正在加载《${playlist.name}》的更多歌曲...`
      : `正在读取《${playlist.name}》...`
  }

  try {
    const result =
      source === 'recommended'
        ? await invoke<KugouPlaylistDetail>('get_kugou_recommended_playlist_detail', {
            playlistId: playlistKey,
            page: nextPage,
            limit: PLATFORM_PLAYLIST_PAGE_SIZE,
          })
        : await invoke<KugouPlaylistDetail>('get_kugou_playlist_detail', {
            listId: playlist.listId,
            page: nextPage,
            limit: PLATFORM_PLAYLIST_PAGE_SIZE,
          })
    if (loadMore && isSamePlaylist && kugouPlaylistDetail.value) {
      const mergedTracks = mergeKugouSearchTracks(
        kugouPlaylistDetail.value.tracks,
        result.tracks,
      )
      kugouPlaylistDetail.value = {
        ...result,
        tracks: mergedTracks,
        totalTrackCount: result.totalTrackCount || kugouPlaylistDetail.value.totalTrackCount,
        message: formatPlatformPlaylistNotice(
          '酷狗',
          result.playlist.name,
          mergedTracks.length,
          result.totalTrackCount || kugouPlaylistDetail.value.totalTrackCount,
          result.truncated,
        ),
      }
    } else {
      kugouPlaylistDetail.value = {
        ...result,
        message: formatPlatformPlaylistNotice(
          '酷狗',
          result.playlist.name,
          result.tracks.length,
          result.totalTrackCount,
          result.truncated,
        ),
      }
    }
    kugouPlaylistDetailPage.value = nextPage
    if (showStatus) {
      kugouLoginNotice.value = kugouPlaylistDetail.value.message
    }
    if (immersiveMode.value) {
      immersivePlaylistSource.value = 'kugou'
    }
  } catch (err) {
    if (!loadMore) {
      kugouPlaylistDetail.value = null
      kugouPlaylistDetailPage.value = 0
    }
    kugouPlaylistError.value = `酷狗歌单歌曲读取失败：${String(err)}`
  } finally {
    kugouPlaylistDetailLoading.value = false
  }
}

async function loadMoreKugouPlaylistTracks() {
  const playlist = kugouActivePlaylist.value
  const source = kugouSelectedContentSource.value === 'recommended' ? 'recommended' : 'personal'
  if (!playlist || !kugouPlaylistHasMore.value || kugouPlaylistDetailLoading.value) {
    return
  }

  await loadKugouPlaylistDetail(playlist, true, true, source)
}

async function openKugouLoginFromLeft() {
  activePanelView.value = 'kugou'
  await startKugouQrLogin()
}

async function refreshKugouPlaylistsFromLeft() {
  activePanelView.value = 'kugou'
  await refreshKugouPlaylists(true)
}

async function openKugouPlaylistFromLeft(playlist: KugouPlaylistSummary) {
  activePanelView.value = 'kugou'
  await loadKugouPlaylistDetail(playlist, true, false, 'personal')
  await scrollPlaylistDetailIntoView('kugou')
}

async function refreshKugouRecommendedPlaylists(showStatus = true, loadMore = false) {
  if (kugouRecommendedPlaylistsLoading.value) {
    return
  }

  const nextPage = loadMore ? kugouRecommendedPlaylistPage.value + 1 : 1
  kugouRecommendedPlaylistsLoading.value = true
  kugouRecommendedPlaylistError.value = ''
  if (!loadMore) {
    kugouRecommendedPlaylistNotice.value = ''
  }
  if (showStatus) {
    kugouRecommendedPlaylistNotice.value = loadMore
      ? '正在加载更多酷狗推荐歌单...'
      : '正在读取酷狗推荐歌单...'
  }

  try {
    const result = await invoke<KugouRecommendedPlaylists>('list_kugou_recommended_playlists', {
      page: nextPage,
      limit: PLATFORM_SEARCH_PAGE_SIZE,
    })
    const playlists = loadMore
      ? mergeKugouPlaylists(kugouRecommendedPlaylists.value, result.playlists)
      : result.playlists
    kugouRecommendedPlaylists.value = playlists
    kugouRecommendedPlaylistPage.value = result.page
    kugouRecommendedPlaylistTotal.value = result.total
    kugouRecommendedPlaylistHasMore.value = result.truncated
    kugouRecommendedPlaylistNotice.value = result.message
    if (!loadMore && playlists.length > 0 && !kugouActivePlaylist.value) {
      await loadKugouPlaylistDetail(playlists[0], false, false, 'recommended')
    }
  } catch (err) {
    kugouRecommendedPlaylistError.value = `酷狗推荐歌单读取失败：${String(err)}`
  } finally {
    kugouRecommendedPlaylistsLoading.value = false
  }
}

async function refreshKugouRecommendedPlaylistsFromLeft(loadMore = false) {
  activePanelView.value = 'kugou'
  await refreshKugouRecommendedPlaylists(true, loadMore)
}

async function openKugouRecommendedPlaylistFromLeft(playlist: KugouPlaylistSummary) {
  activePanelView.value = 'kugou'
  await loadKugouPlaylistDetail(playlist, true, false, 'recommended')
  await scrollPlaylistDetailIntoView('kugou')
}

async function loadKugouDailyRecommendations(showStatus = true) {
  if (kugouDailyRecommendationLoading.value) {
    return
  }

  kugouDailyRecommendationLoading.value = true
  kugouDailyRecommendationError.value = ''
  if (showStatus) {
    kugouDailyRecommendationNotice.value = '正在读取酷狗每日推荐...'
  }

  try {
    const result = await invoke<KugouSearchResult>('get_kugou_daily_recommended_songs')
    kugouDailyRecommendation.value = result
    kugouDailyRecommendationNotice.value = result.message
    if (immersiveMode.value) {
      immersivePlaylistSource.value = 'kugou'
    }
  } catch (err) {
    kugouDailyRecommendationError.value = `酷狗每日推荐读取失败：${String(err)}`
  } finally {
    kugouDailyRecommendationLoading.value = false
  }
}

function resetKugouPlaylistState() {
  const keepRecommendedDetail = kugouSelectedContentSource.value === 'recommended'
  kugouPlaylists.value = []
  kugouSelectedPlaylistId.value = ''
  if (!keepRecommendedDetail) {
    kugouSelectedContentSource.value = ''
    kugouPlaylistDetail.value = null
    kugouPlaylistDetailPage.value = 0
  }
  kugouPlaylistsLoading.value = false
  kugouPlaylistDetailLoading.value = false
  kugouPlaylistError.value = ''
  kugouDailyRecommendation.value = null
  kugouDailyRecommendationLoading.value = false
  kugouDailyRecommendationError.value = ''
  kugouDailyRecommendationNotice.value = ''
}

async function searchKugouSongs(loadMore = false) {
  const keyword = loadMore
    ? (kugouSearchResult.value?.keyword ?? kugouSearchQuery.value).trim()
    : kugouSearchQuery.value.trim()
  if (!keyword) {
    kugouSearchError.value = '请输入酷狗搜索关键词。'
    return
  }

  kugouSearchLoading.value = true
  kugouSearchError.value = ''
  const nextPage = loadMore ? kugouSearchPage.value + 1 : 1
  kugouSearchNotice.value = loadMore
    ? `正在加载“${keyword}”的更多酷狗结果...`
    : `正在搜索“${keyword}”...`

  try {
    const result = await invoke<KugouSearchResult>('search_kugou_songs', {
      keyword,
      page: nextPage,
      limit: PLATFORM_SEARCH_PAGE_SIZE,
    })
    if (loadMore && kugouSearchResult.value?.keyword === result.keyword) {
      const mergedTracks = mergeKugouSearchTracks(
        kugouSearchResult.value.tracks,
        result.tracks,
      )
      kugouSearchResult.value = {
        ...result,
        tracks: mergedTracks,
        total: result.total || kugouSearchResult.value.total,
      }
    } else {
      kugouSearchResult.value = result
    }
    kugouSearchPage.value = nextPage
    kugouSearchNotice.value = formatPlatformSearchNotice(
      '酷狗',
      result.keyword,
      kugouSearchResult.value.tracks.length,
      kugouSearchResult.value.total,
    )
    if (immersiveMode.value) {
      immersivePlaylistSource.value = 'kugou'
    }
  } catch (err) {
    if (!loadMore) {
      kugouSearchResult.value = null
      kugouSearchPage.value = 0
    }
    kugouSearchError.value = `酷狗搜索失败：${String(err)}`
  } finally {
    kugouSearchLoading.value = false
  }
}

async function playKugouTrack(
  track: KugouSearchTrack,
  contextTracks?: KugouSearchTrack[],
  options: OnlinePlaybackOptions = {},
) {
  const sourceTracks = resolveKugouPlaybackTracks(track, contextTracks)
  kugouTrackActionHash.value = track.hash
  kugouSearchError.value = ''
  playerError.value = ''
  playerStatus.value = `正在获取酷狗《${track.name}》的在线播放地址并创建本机播放代理...`

  try {
    await applyKugouQualityAvailabilityBeforePlayback(track)
    const { playback, cached } = await resolveKugouTrackPlayback(track)
    clearKugouTrackUnavailable(track)
    const onlineTrack = createKugouMusicTrack(track, playback)
    pausePlayback()
    currentIndex.value = -1
    neteaseCurrentTrack.value = onlineTrack
    playbackContext.value = { source: 'kugou', tracks: sourceTracks }
    if (immersiveMode.value) {
      immersivePlaylistSource.value = 'kugou'
    }
    currentTime.value = 0
    visualPlaybackTime.value = 0
    duration.value = onlineTrack.duration ?? 0
    await nextTick()
    await playCurrent(true)
    if (!playerError.value) {
      const proxyDetail = playback.proxyDiagnostic ? ` ${playback.proxyDiagnostic}` : ''
      playerStatus.value = cached
        ? `正在通过已预取的本机代理播放酷狗《${track.name}》。${proxyDetail}`
        : `正在通过本机代理播放酷狗《${track.name}》。${proxyDetail}`
      void showKugouLyrics(track, false)
      scheduleOnlinePlaybackPrefetch()
    }
  } catch (err) {
    const reason = normalizeOnlinePlaybackFailureReason(err)
    if (isUnavailableOnlinePlaybackFailure(err)) {
      const key = kugouPlaybackCacheKey(track)
      if (key) {
        markOnlineTrackUnavailable(kugouUnavailableTracks, key, reason)
      }
      if (options.autoSkip && await skipUnavailableKugouTrack(track, sourceTracks, reason)) {
        return
      }
    }
    playerStatus.value = ''
    playerError.value = formatKugouPlaybackFailure(err)
  } finally {
    kugouTrackActionHash.value = ''
  }
}

function formatKugouPlaybackFailure(err: unknown) {
  const rawMessage = String(err)
  if (!rawMessage.trim()) {
    return '酷狗在线播放失败：未返回具体失败原因。'
  }

  if (
    rawMessage.includes('需要会员/付费授权') ||
    rawMessage.includes('没有拿到完整播放授权') ||
    rawMessage.includes('登录态没有拿到完整播放授权') ||
    (rawMessage.includes('需要付费') && rawMessage.includes('试听片段'))
  ) {
    return '酷狗在线播放失败：当前酷狗账号在项目内没有拿到完整播放授权。项目已尝试普通接口、设备注册后的播放接口和登录态会员接口，但最终只拿到试听片段，已停止播放。你的账号有会员不代表第三方接口一定能拿到与官方客户端相同的设备/风控授权；如果官方客户端可播但这里仍失败，下一步需要做官方客户端或网页播放兜底。'
  }

  return `酷狗在线播放失败：${rawMessage}`
}

async function showKugouLyrics(track: KugouSearchTrack, showStatus = true) {
  kugouLyricsTrack.value = track
  kugouLyricsLoading.value = true
  kugouLyricsError.value = ''
  if (showStatus) {
    kugouSearchNotice.value = `正在读取《${track.name}》酷狗歌词...`
  }

  try {
    kugouLyricsResult.value = await invoke<KugouLyricsResult>('read_kugou_lyrics', {
      hash: track.hash,
      name: track.name,
      artist: formatKugouTrackArtists(track),
      durationMs: track.durationMs ?? null,
    })
    if (showStatus) {
      kugouSearchNotice.value = `已读取《${track.name}》酷狗歌词。`
    }
  } catch (err) {
    kugouLyricsResult.value = null
    kugouLyricsError.value = `酷狗歌词读取失败：${String(err)}`
  } finally {
    kugouLyricsLoading.value = false
  }
}

function closeKugouLyrics() {
  kugouLyricsTrack.value = null
  kugouLyricsResult.value = null
  kugouLyricsLoading.value = false
  kugouLyricsError.value = ''
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
    coverImgUrl: track.coverImgUrl ?? null,
    neteaseSongId: track.id,
    category: '网易云',
    tags: [],
    url: playback.url,
    duration: durationSeconds,
    playbackLevel: normalizePlaybackLevel(playback.level),
    playbackBitrate: normalizePlaybackBitrate(playback.bitrate),
    playbackFileType: normalizePlaybackFileType(playback.fileType),
    playbackSize: normalizePlaybackSize(playback.size),
    favorite: false,
    playCount: 0,
    lastPlayedAt: null,
    playHistory: [],
  }
}

function createKugouMusicTrack(track: KugouSearchTrack, playback: KugouPlaybackUrl): MusicTrack {
  const durationSeconds =
    sanitizeTrackDuration((playback.durationMs ?? track.durationMs ?? 0) / 1000) ?? null

  return {
    id: `${KUGOU_TRACK_ID_PREFIX}:${track.hash}`,
    title: normalizeTrackTitle(track.name) || '酷狗歌曲',
    artist: formatKugouTrackArtists(track),
    album: normalizeTrackAlbum(track.album),
    path: `${KUGOU_TRACK_ID_PREFIX}:${track.hash}`,
    sourcePath: `${KUGOU_TRACK_ID_PREFIX}:${track.hash}`,
    source: 'kugou',
    coverImgUrl: track.coverImgUrl ?? null,
    kugouSongHash: track.hash,
    category: '酷狗',
    tags: [],
    url: playback.url,
    duration: durationSeconds,
    playbackLevel: normalizePlaybackLevel(playback.qualityLevel ?? playback.qualityLabel),
    playbackBitrate: normalizePlaybackBitrate(playback.bitrate),
    playbackFileType: normalizePlaybackFileType(playback.fileType),
    playbackSize: normalizePlaybackSize(playback.size),
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
    const metadataCoverImgUrl = normalizeCoverImgUrl(metadata.coverImgUrl)
    const metadataDuration = sanitizeTrackDuration(metadata.duration)
    const metadataApplied = Boolean(
      metadataTitle || metadataArtist || metadataAlbum || metadataCoverImgUrl || metadataDuration,
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
        coverImgUrl: metadataCoverImgUrl,
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

function createDefaultMusicStageTunings(): MusicStageTuningMap {
  return {
    galaxy: { ...DEFAULT_MUSIC_STAGE_TUNING },
    dj: { ...DEFAULT_MUSIC_STAGE_TUNING },
  }
}

function stageTuningOptionsForPreset(preset: MusicVisualStagePreset): MusicStageTuningOption[] {
  return preset === 'galaxy' ? galaxyStageTuningOptions : djStageTuningOptions
}

function normalizeMusicStageTuning(
  value?: Partial<MusicStageTuning> | null,
  preset: MusicVisualStagePreset = 'dj',
): MusicStageTuning {
  const next: MusicStageTuning = { ...DEFAULT_MUSIC_STAGE_TUNING }

  for (const option of stageTuningOptionsForPreset(preset)) {
    const rawValue = value?.[option.key]
    next[option.key] =
      typeof rawValue === 'number' ? clamp(rawValue, option.min, option.max) : DEFAULT_MUSIC_STAGE_TUNING[option.key]
  }
  next.centerPulse = false

  return next
}

function normalizeMusicStageTunings(
  value?: Partial<Record<string, Partial<MusicStageTuning>>> | null,
  legacyValue?: Partial<MusicStageTuning> | null,
): MusicStageTuningMap {
  const source = value ?? {}
  const legacyDjValue = source.dj ?? legacyValue

  return {
    galaxy: normalizeMusicStageTuning(source.galaxy, 'galaxy'),
    dj: normalizeMusicStageTuning(legacyDjValue, 'dj'),
  }
}

function formatStageTuningValue(value: number) {
  return `${Math.round(clamp(value, 0, 3) * 100)}%`
}

function resetVisualStageTuning() {
  visualStageTunings.value = {
    ...visualStageTunings.value,
    [visualStagePreset.value]: { ...DEFAULT_MUSIC_STAGE_TUNING },
  }
}

function lyricStageDefaultsForPreset(preset: MusicLyricStagePreset) {
  return LYRIC_STAGE_PRESET_DEFAULTS[preset] ?? LYRIC_STAGE_PRESET_DEFAULTS.projection
}

function resetLyricStageParameters() {
  const defaults = lyricStageDefaultsForPreset('projection')
  lyricStageTilt.value = defaults.tilt
  lyricStageGlow.value = defaults.glow
  lyricStageFontScale.value = defaults.fontScale
  lyricStageVertical.value = defaults.vertical
  lyricStageDistance.value = LYRIC_STAGE_DISTANCE_DEFAULT
  lyricStageSideOpacity.value = defaults.sideOpacity
}

function lyricFontScaleValue() {
  return (
    LYRIC_STAGE_FONT_SCALE_MIN +
    clamp(lyricStageFontScale.value, 0, 1) * (LYRIC_STAGE_FONT_SCALE_MAX - LYRIC_STAGE_FONT_SCALE_MIN)
  )
}

function lyricVerticalOffsetPx() {
  return Math.round((clamp(lyricStageVertical.value, 0, 1) - 0.5) * LYRIC_STAGE_VERTICAL_OFFSET_SPAN_PX)
}

function lyricDistanceOffsetPx() {
  return Math.round((clamp(lyricStageDistance.value, 0, 1) - 0.5) * LYRIC_STAGE_DISTANCE_OFFSET_SPAN_PX)
}

function lyricDistanceScaleValue() {
  return (
    LYRIC_STAGE_DISTANCE_SCALE_MIN +
    clamp(lyricStageDistance.value, 0, 1) * (LYRIC_STAGE_DISTANCE_SCALE_MAX - LYRIC_STAGE_DISTANCE_SCALE_MIN)
  )
}

function lyricSideOpacityValue() {
  return 0.32 + clamp(lyricStageSideOpacity.value, 0, 1) * 0.46
}

function isMusicVisualMode(value?: string | null): value is MusicVisualMode {
  return visualModeOptions.some((option) => option.value === value)
}

function isMusicVisualStagePreset(value?: string | null): value is MusicVisualStagePreset {
  return stagePresetOptions.some((option) => option.value === value)
}

function normalizeUnitSetting(value: unknown, fallback: number) {
  return typeof value === 'number' ? clamp(value, 0, 1) : clamp(fallback, 0, 1)
}

function applyVisualStagePreset(preset: MusicVisualStagePreset) {
  const option = stagePresetOptions.find((item) => item.value === preset) ?? defaultVisualStagePresetOption
  visualStagePreset.value = option.value
  visualMode.value = option.mode
  visualSpectrumStyle.value = option.spectrumStyle
  visualLineStyle.value = option.lineStyle
  visualRippleStyle.value = option.rippleStyle
}

function setVisualStagePreset(preset: MusicVisualStagePreset) {
  applyVisualStagePreset(preset)
}

function isMusicSpectrumStyle(value?: string | null): value is MusicSpectrumStyle {
  return spectrumStyleOptions.some((option) => option.value === value)
}

function handleWebglStarfieldUnavailable(reason: string) {
  webglStarfieldUnavailable.value = true
  console.warn(reason)
}

function isMusicLineStyle(value?: string | null): value is MusicLineStyle {
  return lineStyleOptions.some((option) => option.value === value)
}

function isMusicRippleStyle(value?: string | null): value is MusicRippleStyle {
  return rippleStyleOptions.some((option) => option.value === value)
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

function allLocalTrackIds() {
  return tracks.value.map((track) => track.id)
}

function uniqueLocalTrackIds(trackIds: string[]) {
  const seenTrackIds = new Set<string>()
  return trackIds
    .map((trackId) => normalizeTrackId(trackId))
    .filter((trackId) => {
      if (!trackId || seenTrackIds.has(trackId) || !trackById(trackId)) {
        return false
      }

      seenTrackIds.add(trackId)
      return true
    })
}

function visibleLocalTrackIds() {
  if (activePanelView.value !== 'library') {
    return []
  }

  return uniqueLocalTrackIds(filteredTracks.value.map((track) => track.id))
}

function localPlaybackTrackIds() {
  const contextTrackIds =
    playbackContext.value.source === 'local'
      ? uniqueLocalTrackIds(playbackContext.value.trackIds)
      : []

  return contextTrackIds.length > 0 ? contextTrackIds : allLocalTrackIds()
}

function currentLocalImmersiveSourceTracks() {
  const sourceTracks = localPlaybackTrackIds()
    .map((trackId) => trackById(trackId))
    .filter((track): track is MusicTrack => Boolean(track))

  return sourceTracks.length > 0 ? sourceTracks : tracks.value
}

function centeredPlaybackWindow<T>(sourceTracks: T[], currentListIndex: number) {
  if (sourceTracks.length <= IMMERSIVE_PLAYLIST_FALLBACK_LIMIT) {
    return sourceTracks
  }

  if (currentListIndex < 0) {
    return sourceTracks.slice(0, IMMERSIVE_PLAYLIST_FALLBACK_LIMIT)
  }

  const rawStart = currentListIndex - IMMERSIVE_PLAYLIST_CONTEXT_RADIUS
  const rawEnd = currentListIndex + IMMERSIVE_PLAYLIST_CONTEXT_RADIUS + 1
  const start = Math.max(0, Math.min(rawStart, sourceTracks.length - IMMERSIVE_PLAYLIST_FALLBACK_LIMIT))
  const end = Math.min(sourceTracks.length, Math.max(rawEnd, start + IMMERSIVE_PLAYLIST_FALLBACK_LIMIT))
  return sourceTracks.slice(start, end)
}

function resolveLocalPlaybackTrackIds(trackId: string, contextTrackIds?: string[]) {
  const explicitTrackIds = uniqueLocalTrackIds(contextTrackIds ?? [])
  if (explicitTrackIds.includes(trackId)) {
    return explicitTrackIds
  }

  const visibleTrackIds = visibleLocalTrackIds()
  if (visibleTrackIds.includes(trackId)) {
    return visibleTrackIds
  }

  const queuedTrackIds = uniqueLocalTrackIds([trackId, ...playQueue.value])
  if (activeLibraryView.value === 'queue' && queuedTrackIds.includes(trackId)) {
    return queuedTrackIds
  }

  const fallbackTrackIds = allLocalTrackIds()
  return fallbackTrackIds.includes(trackId) ? fallbackTrackIds : []
}

function setLocalPlaybackContext(trackId: string, contextTrackIds?: string[]) {
  playbackContext.value = {
    source: 'local',
    trackIds: resolveLocalPlaybackTrackIds(trackId, contextTrackIds),
  }
}

function uniqueNeteasePlaybackTracks(trackList: NeteasePlaylistTrack[]) {
  const seenSongIds = new Set<number>()
  return trackList.filter((track) => {
    if (!Number.isFinite(track.id) || seenSongIds.has(track.id)) {
      return false
    }

    seenSongIds.add(track.id)
    return true
  })
}

function resolveNeteasePlaybackTracks(
  track: NeteasePlaylistTrack,
  contextTracks?: NeteasePlaylistTrack[],
) {
  const explicitTracks = uniqueNeteasePlaybackTracks(contextTracks ?? [])
  if (explicitTracks.some((candidate) => candidate.id === track.id)) {
    return explicitTracks
  }

  const playlistTracks = uniqueNeteasePlaybackTracks(neteasePlaylistDetail.value?.tracks ?? [])
  if (playlistTracks.some((candidate) => candidate.id === track.id)) {
    return playlistTracks
  }

  const searchTracks = uniqueNeteasePlaybackTracks(neteaseSearchTracks.value)
  if (searchTracks.some((candidate) => candidate.id === track.id)) {
    return searchTracks
  }

  return uniqueNeteasePlaybackTracks([track, ...explicitTracks])
}

function currentNeteasePlaybackTracks() {
  const currentSongId = currentTrack.value?.neteaseSongId
  const playlistTracks = uniqueNeteasePlaybackTracks(neteasePlaylistDetail.value?.tracks ?? [])
  if (currentSongId && playlistTracks.some((track) => track.id === currentSongId)) {
    return playlistTracks
  }

  if (playbackContext.value.source === 'netease') {
    return uniqueNeteasePlaybackTracks(playbackContext.value.tracks)
  }

  const sourceTracks = uniqueNeteasePlaybackTracks([
    ...neteaseSearchTracks.value,
    ...playlistTracks,
  ])
  return currentSongId && sourceTracks.some((track) => track.id === currentSongId)
    ? sourceTracks
    : []
}

function currentNeteaseImmersiveSourceTracks() {
  const playbackTracks = currentNeteasePlaybackTracks()
  if (currentTrack.value?.source === 'netease' && playbackTracks.length > 0) {
    return playbackTracks
  }

  const playlistTracks = uniqueNeteasePlaybackTracks(neteasePlaylistDetail.value?.tracks ?? [])
  if (playlistTracks.length > 0) {
    return playlistTracks
  }

  const searchTracks = uniqueNeteasePlaybackTracks(neteaseSearchTracks.value)
  return searchTracks.length > 0 ? searchTracks : playbackTracks
}

function uniqueKugouPlaybackTracks(trackList: KugouSearchTrack[]) {
  const seenHashes = new Set<string>()
  return trackList.filter((track) => {
    const hash = track.hash.trim()
    if (!hash || seenHashes.has(hash)) {
      return false
    }

    seenHashes.add(hash)
    return true
  })
}

function kugouPlaylistKey(playlist: KugouPlaylistSummary) {
  return String(playlist.globalCollectionId || playlist.id || playlist.listId || '').trim()
}

function mergeKugouPlaylists(
  currentPlaylists: KugouPlaylistSummary[],
  nextPlaylists: KugouPlaylistSummary[],
) {
  const seenKeys = new Set(currentPlaylists.map(kugouPlaylistKey).filter(Boolean))
  const merged = [...currentPlaylists]
  for (const playlist of nextPlaylists) {
    const key = kugouPlaylistKey(playlist)
    if (!key || seenKeys.has(key)) {
      continue
    }

    seenKeys.add(key)
    merged.push(playlist)
  }

  return merged
}

function resolveKugouPlaybackTracks(track: KugouSearchTrack, contextTracks?: KugouSearchTrack[]) {
  const explicitTracks = uniqueKugouPlaybackTracks(contextTracks ?? [])
  if (explicitTracks.some((candidate) => candidate.hash === track.hash)) {
    return explicitTracks
  }

  const playlistTracks = uniqueKugouPlaybackTracks(kugouPlaylistDetail.value?.tracks ?? [])
  if (playlistTracks.some((candidate) => candidate.hash === track.hash)) {
    return playlistTracks
  }

  const searchTracks = uniqueKugouPlaybackTracks(kugouSearchTracks.value)
  if (searchTracks.some((candidate) => candidate.hash === track.hash)) {
    return searchTracks
  }

  return uniqueKugouPlaybackTracks([track, ...explicitTracks])
}

function currentKugouPlaybackTracks() {
  const currentHash = currentTrack.value?.kugouSongHash
  const playlistTracks = uniqueKugouPlaybackTracks(kugouPlaylistDetail.value?.tracks ?? [])
  if (currentHash && playlistTracks.some((track) => track.hash === currentHash)) {
    return playlistTracks
  }

  if (playbackContext.value.source === 'kugou') {
    return uniqueKugouPlaybackTracks(playbackContext.value.tracks)
  }

  const sourceTracks = uniqueKugouPlaybackTracks([
    ...playlistTracks,
    ...kugouDailyRecommendationTracks.value,
    ...kugouSearchTracks.value,
  ])
  return currentHash && sourceTracks.some((track) => track.hash === currentHash) ? sourceTracks : []
}

function resolveCurrentKugouQualityTrack() {
  const currentHash = currentTrack.value?.kugouSongHash
  if (!currentHash) {
    return null
  }

  const sourceTracks = currentKugouPlaybackTracks()
  return sourceTracks.find((track) => track.hash === currentHash) ?? null
}

function currentKugouImmersiveSourceTracks() {
  const playbackTracks = currentKugouPlaybackTracks()
  if (currentTrack.value?.source === 'kugou' && playbackTracks.length > 0) {
    return playbackTracks
  }

  const playlistTracks = uniqueKugouPlaybackTracks(kugouPlaylistDetail.value?.tracks ?? [])
  if (playlistTracks.length > 0) {
    return playlistTracks
  }

  const dailyTracks = uniqueKugouPlaybackTracks(kugouDailyRecommendationTracks.value)
  if (dailyTracks.length > 0) {
    return dailyTracks
  }

  const searchTracks = uniqueKugouPlaybackTracks(kugouSearchTracks.value)
  return searchTracks.length > 0 ? searchTracks : playbackTracks
}

async function maybeLoadMoreNeteaseTracksForPlayback(sourceTracks: NeteasePlaylistTrack[]) {
  const currentSongId = currentTrack.value?.neteaseSongId
  const playlist = neteaseSelectedPlaylist.value
  if (
    !currentSongId ||
    !playlist ||
    !neteasePlaylistHasMore.value ||
    neteasePlaylistDetailLoading.value ||
    !neteasePlaylistDetail.value?.tracks.some((track) => track.id === currentSongId)
  ) {
    return sourceTracks
  }

  const currentListIndex = sourceTracks.findIndex((track) => track.id === currentSongId)
  if (
    currentListIndex >= 0 &&
    currentListIndex < sourceTracks.length - PLAYBACK_CONTEXT_PREFETCH_THRESHOLD
  ) {
    return sourceTracks
  }

  const beforeCount = neteasePlaylistDetail.value.tracks.length
  await loadNeteasePlaylistDetail(playlist, false, true)
  const expandedTracks = currentNeteasePlaybackTracks()
  return expandedTracks.length > beforeCount ? expandedTracks : sourceTracks
}

async function maybeLoadMoreKugouTracksForPlayback(sourceTracks: KugouSearchTrack[]) {
  const currentHash = currentTrack.value?.kugouSongHash
  const playlist = kugouActivePlaylist.value
  const source = kugouSelectedContentSource.value === 'recommended' ? 'recommended' : 'personal'
  if (
    !currentHash ||
    !playlist ||
    !kugouPlaylistHasMore.value ||
    kugouPlaylistDetailLoading.value ||
    !kugouPlaylistDetail.value?.tracks.some((track) => track.hash === currentHash)
  ) {
    return sourceTracks
  }

  const currentListIndex = sourceTracks.findIndex((track) => track.hash === currentHash)
  if (
    currentListIndex >= 0 &&
    currentListIndex < sourceTracks.length - PLAYBACK_CONTEXT_PREFETCH_THRESHOLD
  ) {
    return sourceTracks
  }

  const beforeCount = kugouPlaylistDetail.value.tracks.length
  await loadKugouPlaylistDetail(playlist, false, true, source)
  const expandedTracks = currentKugouPlaybackTracks()
  return expandedTracks.length > beforeCount ? expandedTracks : sourceTracks
}

function clearOnlinePlaybackPrefetchTimer() {
  onlinePlaybackPrefetchSerial += 1
  if (onlinePlaybackPrefetchTimer !== null) {
    window.clearTimeout(onlinePlaybackPrefetchTimer)
    onlinePlaybackPrefetchTimer = null
  }
}

function clearOnlineStallRecoveryTimer() {
  onlineStallRecoverySerial += 1
  if (onlineStallRecoveryTimer !== null) {
    window.clearTimeout(onlineStallRecoveryTimer)
    onlineStallRecoveryTimer = null
  }
}

function resetOnlineStallRecovery() {
  clearOnlineStallRecoveryTimer()
  onlineStallRecoveryAttempts = 0
  onlineStallStartedAt = 0
}

function scheduleOnlinePlaybackPrefetch() {
  clearOnlinePlaybackPrefetchTimer()
  const candidate = nextOnlinePlaybackPrefetchCandidate()
  if (!candidate) {
    return
  }

  const serial = onlinePlaybackPrefetchSerial
  onlinePlaybackPrefetchTimer = window.setTimeout(() => {
    onlinePlaybackPrefetchTimer = null
    void prefetchOnlinePlayback(candidate, serial)
  }, ONLINE_PLAYBACK_PREFETCH_DELAY_MS)
}

function nextOnlinePlaybackPrefetchCandidate(): OnlinePlaybackPrefetchCandidate | null {
  if (repeatMode.value === 'one' || shuffleEnabled.value) {
    return null
  }

  if (currentTrack.value?.source === 'netease') {
    const sourceTracks = currentNeteasePlaybackTracks()
    const currentSongId = currentTrack.value.neteaseSongId
    const currentListIndex = sourceTracks.findIndex((track) => track.id === currentSongId)
    const targetIndex = firstPlayableNeteaseCandidateIndex(sourceTracks, currentListIndex, false)
    return targetIndex !== undefined ? { source: 'netease', track: sourceTracks[targetIndex] } : null
  }

  if (currentTrack.value?.source === 'kugou') {
    const sourceTracks = currentKugouPlaybackTracks()
    const currentHash = currentTrack.value.kugouSongHash
    const currentListIndex = sourceTracks.findIndex((track) => track.hash === currentHash)
    const targetIndex = firstPlayableKugouCandidateIndex(sourceTracks, currentListIndex, false)
    return targetIndex !== undefined ? { source: 'kugou', track: sourceTracks[targetIndex] } : null
  }

  return null
}

async function prefetchOnlinePlayback(candidate: OnlinePlaybackPrefetchCandidate, serial: number) {
  if (serial !== onlinePlaybackPrefetchSerial) {
    return
  }

  try {
    if (candidate.source === 'netease') {
      if (getCachedNeteasePlayback(candidate.track)) {
        return
      }
      const playback = await fetchNeteasePlayback(candidate.track)
      if (serial === onlinePlaybackPrefetchSerial) {
        cacheNeteasePlayback(candidate.track, playback)
      }
      return
    }

    if (getCachedKugouPlayback(candidate.track)) {
      return
    }
    const playback = await fetchKugouPlayback(candidate.track)
    if (serial === onlinePlaybackPrefetchSerial) {
      cacheKugouPlayback(candidate.track, playback)
    }
  } catch {
    // 预取失败不打断当前播放；真正切歌时会按正常播放链路重新获取。
  }
}

function nextPlaybackCandidateIndices(currentListIndex: number, count: number, manual: boolean) {
  if (count <= 0) {
    return []
  }

  const indices: number[] = []
  const visited = new Set<number>()
  let cursor = currentListIndex

  for (let attempt = 0; attempt < count; attempt += 1) {
    const nextIndex = nextPlaybackListIndex(cursor, count, manual)
    if (
      nextIndex < 0 ||
      visited.has(nextIndex) ||
      (nextIndex === currentListIndex && currentListIndex >= 0)
    ) {
      break
    }

    indices.push(nextIndex)
    visited.add(nextIndex)
    cursor = nextIndex
  }

  return indices
}

function firstPlayableNeteaseCandidateIndex(
  sourceTracks: NeteasePlaylistTrack[],
  currentListIndex: number,
  manual: boolean,
) {
  return nextPlaybackCandidateIndices(currentListIndex, sourceTracks.length, manual)
    .find((index) => !neteaseTrackUnavailableReason(sourceTracks[index]))
}

function firstPlayableKugouCandidateIndex(
  sourceTracks: KugouSearchTrack[],
  currentListIndex: number,
  manual: boolean,
) {
  return nextPlaybackCandidateIndices(currentListIndex, sourceTracks.length, manual)
    .find((index) => !kugouTrackUnavailableReason(sourceTracks[index]))
}

async function skipUnavailableNeteaseTrack(
  failedTrack: NeteasePlaylistTrack,
  sourceTracks: NeteasePlaylistTrack[],
  reason: string,
) {
  const currentListIndex = sourceTracks.findIndex((track) => track.id === failedTrack.id)
  const targetIndex = firstPlayableNeteaseCandidateIndex(sourceTracks, currentListIndex, false)
  if (targetIndex === undefined) {
    playerStatus.value = ''
    playerError.value = `网易云《${failedTrack.name}》当前不可播放：${reason}。后续没有可自动尝试的歌曲。`
    return true
  }

  const nextTrack = sourceTracks[targetIndex]
  playerStatus.value = `已跳过网易云《${failedTrack.name}》：${reason}。正在尝试下一首《${nextTrack.name}》...`
  await playNeteaseTrack(nextTrack, sourceTracks, { autoSkip: true })
  return true
}

async function skipUnavailableKugouTrack(
  failedTrack: KugouSearchTrack,
  sourceTracks: KugouSearchTrack[],
  reason: string,
) {
  const currentListIndex = sourceTracks.findIndex((track) => track.hash === failedTrack.hash)
  const targetIndex = firstPlayableKugouCandidateIndex(sourceTracks, currentListIndex, false)
  if (targetIndex === undefined) {
    playerStatus.value = ''
    playerError.value = `酷狗《${failedTrack.name}》当前不可播放：${reason}。后续没有可自动尝试的歌曲。`
    return true
  }

  const nextTrack = sourceTracks[targetIndex]
  playerStatus.value = `已跳过酷狗《${failedTrack.name}》：${reason}。正在尝试下一首《${nextTrack.name}》...`
  await playKugouTrack(nextTrack, sourceTracks, { autoSkip: true })
  return true
}

function previousPlaybackListIndex(currentListIndex: number, count: number) {
  if (count === 0) {
    return -1
  }

  return currentListIndex <= 0 ? count - 1 : currentListIndex - 1
}

function nextPlaybackListIndex(currentListIndex: number, count: number, manual: boolean) {
  if (count === 0) {
    return -1
  }

  if (shuffleEnabled.value && count > 1) {
    let next = currentListIndex
    while (next === currentListIndex) {
      next = Math.floor(Math.random() * count)
    }
    return next
  }

  const next = currentListIndex + 1
  if (next < count) {
    return next
  }

  if (manual || repeatMode.value === 'all') {
    return 0
  }

  return -1
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
    pausePlayback()
    return
  }

  await playCurrent()
}

async function playTrack(index: number, contextTrackIds?: string[]) {
  if (index < 0 || index >= tracks.value.length) {
    return
  }

  const track = tracks.value[index]
  neteaseCurrentTrack.value = null
  setLocalPlaybackContext(track.id, contextTrackIds)
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
    const contextTrackIds = resolveLocalPlaybackTrackIds(trackId)
    if (activeLibraryView.value === 'queue') {
      removeTrackFromQueue(trackId)
    }
    await playTrack(index, contextTrackIds)
  }
}

function immersiveTrackActive(track: MusicTrack) {
  return currentTrack.value?.id === track.id
}

function immersiveNeteaseTrackActive(track: NeteasePlaylistTrack) {
  return currentTrack.value?.source === 'netease' && currentTrack.value.neteaseSongId === track.id
}

function immersiveKugouTrackActive(track: KugouSearchTrack) {
  return currentTrack.value?.source === 'kugou' && currentTrack.value.kugouSongHash === track.hash
}

function canPlayImmersiveTrack(track: MusicTrack) {
  return track.source !== 'netease' && track.source !== 'kugou' && !immersiveTrackActive(track)
}

function canPlayImmersiveNeteaseTrack(track: NeteasePlaylistTrack) {
  return neteaseLoggedIn.value && !immersiveNeteaseTrackActive(track) && neteaseTrackActionId.value !== track.id
}

function canPlayImmersiveKugouTrack(track: KugouSearchTrack) {
  return !immersiveKugouTrackActive(track) && kugouTrackActionHash.value !== track.hash
}

async function playImmersiveTrack(track: MusicTrack) {
  if (!canPlayImmersiveTrack(track)) {
    return
  }

  immersivePlaylistSource.value = 'local'
  const trackIndex = tracks.value.findIndex((candidate) => candidate.id === track.id)
  if (trackIndex < 0) {
    return
  }

  await playTrack(
    trackIndex,
    currentLocalImmersiveSourceTracks().map((candidate) => candidate.id),
  )
}

async function playImmersiveNeteaseTrack(track: NeteasePlaylistTrack) {
  if (!canPlayImmersiveNeteaseTrack(track)) {
    return
  }

  immersivePlaylistSource.value = 'netease'
  await playNeteaseTrack(track, currentNeteaseImmersiveSourceTracks())
}

async function playImmersiveKugouTrack(track: KugouSearchTrack) {
  if (!canPlayImmersiveKugouTrack(track)) {
    return
  }

  immersivePlaylistSource.value = 'kugou'
  await playKugouTrack(track, currentKugouImmersiveSourceTracks())
}

function toggleImmersivePlaylistVisible() {
  immersivePlaylistVisible.value = !immersivePlaylistVisible.value
}

function toggleImmersiveRhythmPanelVisible() {
  immersiveRhythmPanelVisible.value = !immersiveRhythmPanelVisible.value
}

function setImmersiveStageOnlyMode(nextStageOnlyMode: boolean) {
  immersiveStageOnlyMode.value = nextStageOnlyMode
}

function toggleImmersiveStageOnlyMode() {
  setImmersiveStageOnlyMode(!immersiveStageOnlyMode.value)
}

async function playCurrent(resetTime = false) {
  if (!audio.value || !currentTrack.value) {
    return
  }

  const track = currentTrack.value
  const requestSerial = ++playbackRequestSerial
  try {
    if (resetTime) {
      audio.value.currentTime = 0
    }
    await audio.value.play()
    if (requestSerial !== playbackRequestSerial || track !== currentTrack.value) {
      return
    }
    playing.value = true
    if (immersiveMode.value) {
      void prepareImmersiveVisualization()
    }
    if (resetTime && track === currentTrack.value) {
      recordTrackPlayback(track)
    }
  } catch (err) {
    if (
      requestSerial !== playbackRequestSerial ||
      track !== currentTrack.value ||
      isInterruptedPlayError(err)
    ) {
      return
    }
    playing.value = false
    playerError.value = `无法播放该音频：${String(err)}`
  }
}

function waitForAudioReadyForPlayback(player: HTMLAudioElement, timeoutMs = 2500) {
  if (player.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA) {
    return Promise.resolve()
  }

  return new Promise<void>((resolve) => {
    let settled = false
    let timer: number | null = null
    const cleanup = () => {
      player.removeEventListener('loadedmetadata', settle)
      player.removeEventListener('loadeddata', settle)
      player.removeEventListener('canplay', settle)
      player.removeEventListener('error', settle)
      if (timer !== null) {
        window.clearTimeout(timer)
        timer = null
      }
    }
    const settle = () => {
      if (settled) {
        return
      }
      settled = true
      cleanup()
      resolve()
    }

    player.addEventListener('loadedmetadata', settle)
    player.addEventListener('loadeddata', settle)
    player.addEventListener('canplay', settle)
    player.addEventListener('error', settle)
    timer = window.setTimeout(settle, timeoutMs)
  })
}

async function loadAndPlayCurrentAudioAt(
  track: MusicTrack,
  resumeTime: number,
  sourceOverride?: string,
) {
  if (!audio.value || !sameMusicTrackIdentity(currentTrack.value, track)) {
    return false
  }

  const player = audio.value
  const requestSerial = ++playbackRequestSerial
  const nextSource = sourceOverride?.trim() || track.url
  if (nextSource && player.src !== nextSource) {
    player.src = nextSource
  }

  player.load()
  await waitForAudioReadyForPlayback(player)
  await nextTick()

  if (
    requestSerial !== playbackRequestSerial ||
    !sameMusicTrackIdentity(currentTrack.value, track)
  ) {
    return false
  }

  const nextTime = Math.max(resumeTime, 0)
  try {
    player.currentTime = nextTime
    currentTime.value = nextTime
    visualPlaybackTime.value = nextTime
  } catch {
    currentTime.value = 0
    visualPlaybackTime.value = 0
  }

  try {
    await player.play()
  } catch (err) {
    if (
      requestSerial !== playbackRequestSerial ||
      !sameMusicTrackIdentity(currentTrack.value, track) ||
      isInterruptedPlayError(err)
    ) {
      return false
    }
    playing.value = false
    playerError.value = `无法播放该音频：${String(err)}`
    return false
  }

  if (
    requestSerial !== playbackRequestSerial ||
    !sameMusicTrackIdentity(currentTrack.value, track)
  ) {
    return false
  }

  playing.value = true
  playerError.value = ''
  if (immersiveMode.value) {
    void prepareImmersiveVisualization()
  }
  return true
}

function invalidatePendingPlayback() {
  playbackRequestSerial += 1
}

function isInterruptedPlayError(err: unknown) {
  if (err instanceof DOMException && err.name === 'AbortError') {
    return true
  }

  const message = String(err)
  return (
    message.includes('AbortError') &&
    (message.includes('pause()') ||
      message.includes('interrupted') ||
      message.includes('new load request'))
  )
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
  invalidatePendingPlayback()
  resetOnlineStallRecovery()
  audio.value?.pause()
  playing.value = false
}

async function playPrevious() {
  if (!currentTrack.value && tracks.value.length === 0) {
    return
  }

  if (currentTrack.value && audio.value && audio.value.currentTime > 4) {
    audio.value.currentTime = 0
    currentTime.value = 0
    return
  }

  if (currentTrack.value?.source === 'netease') {
    await playPreviousNeteaseTrack()
    return
  }

  if (currentTrack.value?.source === 'kugou') {
    await playPreviousKugouTrack()
    return
  }

  const target = previousLocalPlaybackTarget()
  if (target.index >= 0) {
    await playTrack(target.index, target.contextTrackIds)
  }
}

async function playNext(manual = true) {
  if (currentTrack.value?.source === 'netease') {
    await playNextNeteaseTrack(manual)
    return
  }

  if (currentTrack.value?.source === 'kugou') {
    await playNextKugouTrack(manual)
    return
  }

  const target = nextLocalPlaybackTarget(manual)
  if (target.index < 0) {
    stopPlaybackAtListEnd()
    return
  }

  await playTrack(target.index, target.contextTrackIds)
}

function stopPlaybackAtListEnd() {
  pausePlayback()
  if (audio.value) {
    audio.value.currentTime = 0
  }
  currentTime.value = 0
  visualPlaybackTime.value = 0
}

function previousLocalPlaybackTarget() {
  const contextTrackIds = localPlaybackTrackIds()
  const currentTrackId = tracks.value[currentIndex.value]?.id ?? currentTrack.value?.id ?? ''
  const currentListIndex = contextTrackIds.findIndex((trackId) => trackId === currentTrackId)
  const targetListIndex = previousPlaybackListIndex(currentListIndex, contextTrackIds.length)
  const targetTrackId = targetListIndex >= 0 ? contextTrackIds[targetListIndex] : ''
  return {
    index: tracks.value.findIndex((track) => track.id === targetTrackId),
    contextTrackIds,
  }
}

function nextLocalPlaybackTarget(manual: boolean) {
  const queuedIndex = dequeueNextTrackIndex()
  if (queuedIndex >= 0) {
    const queuedTrackId = tracks.value[queuedIndex]?.id ?? ''
    const queuedContextTrackIds = uniqueLocalTrackIds([queuedTrackId, ...playQueue.value])
    return {
      index: queuedIndex,
      contextTrackIds:
        queuedContextTrackIds.length > 0 ? queuedContextTrackIds : localPlaybackTrackIds(),
    }
  }

  const contextTrackIds = localPlaybackTrackIds()
  const currentTrackId = tracks.value[currentIndex.value]?.id ?? currentTrack.value?.id ?? ''
  const currentListIndex = contextTrackIds.findIndex((trackId) => trackId === currentTrackId)
  const targetListIndex = nextPlaybackListIndex(currentListIndex, contextTrackIds.length, manual)
  const targetTrackId = targetListIndex >= 0 ? contextTrackIds[targetListIndex] : ''
  return {
    index: tracks.value.findIndex((track) => track.id === targetTrackId),
    contextTrackIds,
  }
}

async function playPreviousNeteaseTrack() {
  const sourceTracks = currentNeteasePlaybackTracks()
  const currentSongId = currentTrack.value?.neteaseSongId
  const currentListIndex = sourceTracks.findIndex((track) => track.id === currentSongId)
  const targetIndex = previousPlaybackListIndex(currentListIndex, sourceTracks.length)
  if (targetIndex >= 0) {
    await playNeteaseTrack(sourceTracks[targetIndex], sourceTracks)
  }
}

async function playNextNeteaseTrack(manual: boolean) {
  const sourceTracks = await maybeLoadMoreNeteaseTracksForPlayback(currentNeteasePlaybackTracks())
  const currentSongId = currentTrack.value?.neteaseSongId
  const currentListIndex = sourceTracks.findIndex((track) => track.id === currentSongId)
  const targetIndex = firstPlayableNeteaseCandidateIndex(sourceTracks, currentListIndex, manual)
  if (targetIndex === undefined) {
    playerStatus.value = ''
    playerError.value = sourceTracks.length > 0
      ? '网易云后续歌曲当前都不可播放或正在等待重试，请稍后刷新歌单或手动重试。'
      : ''
    stopPlaybackAtListEnd()
    return
  }

  await playNeteaseTrack(sourceTracks[targetIndex], sourceTracks, { autoSkip: !manual })
}

async function playPreviousKugouTrack() {
  const sourceTracks = currentKugouPlaybackTracks()
  const currentHash = currentTrack.value?.kugouSongHash
  const currentListIndex = sourceTracks.findIndex((track) => track.hash === currentHash)
  const targetIndex = previousPlaybackListIndex(currentListIndex, sourceTracks.length)
  if (targetIndex >= 0) {
    await playKugouTrack(sourceTracks[targetIndex], sourceTracks)
  }
}

async function playNextKugouTrack(manual: boolean) {
  const sourceTracks = await maybeLoadMoreKugouTracksForPlayback(currentKugouPlaybackTracks())
  const currentHash = currentTrack.value?.kugouSongHash
  const currentListIndex = sourceTracks.findIndex((track) => track.hash === currentHash)
  const targetIndex = firstPlayableKugouCandidateIndex(sourceTracks, currentListIndex, manual)
  if (targetIndex === undefined) {
    playerStatus.value = ''
    playerError.value = sourceTracks.length > 0
      ? '酷狗后续歌曲当前都不可播放或正在等待重试，请稍后刷新歌单或手动重试。'
      : ''
    stopPlaybackAtListEnd()
    return
  }

  await playKugouTrack(sourceTracks[targetIndex], sourceTracks, { autoSkip: !manual })
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
  visualPlaybackTime.value = time
  if (
    currentTrackOnline.value &&
    onlineStallStartedAt > 0 &&
    time > onlineStallStartedAt + ONLINE_STALL_RECOVERY_PROGRESS_SECONDS
  ) {
    resetOnlineStallRecovery()
    if (
      playerStatus.value.includes('等待缓冲恢复') ||
      playerStatus.value.includes('长时间无数据') ||
      playerStatus.value.includes('重新尝试刷新')
    ) {
      playerStatus.value = ''
    }
  }
}

function handleAudioWaiting() {
  scheduleOnlineStallRecovery('缓冲中')
}

function handleAudioStalled() {
  scheduleOnlineStallRecovery('读取停顿')
}

function handleAudioRecovered() {
  if (!currentTrackOnline.value) {
    return
  }

  const time = audio.value?.currentTime ?? currentTime.value
  if (
    onlineStallStartedAt > 0 &&
    time <= onlineStallStartedAt + ONLINE_STALL_RECOVERY_PROGRESS_SECONDS
  ) {
    return
  }

  resetOnlineStallRecovery()
  if (playerStatus.value.includes('等待缓冲恢复')) {
    playerStatus.value = ''
  }
}

function scheduleOnlineStallRecovery(reason: string) {
  const track = currentTrack.value
  if (!audio.value || !track || !currentTrackOnline.value || !playing.value) {
    return
  }

  if (onlineStallRecoveryTimer !== null) {
    return
  }

  const startedAt = audio.value.currentTime || currentTime.value
  onlineStallStartedAt = startedAt
  const serial = ++onlineStallRecoverySerial
  playerStatus.value = `当前${currentTrackPlatformLabel.value}${reason}，正在等待缓冲恢复...`
  onlineStallRecoveryTimer = window.setTimeout(() => {
    onlineStallRecoveryTimer = null
    void recoverOnlinePlaybackStall(track, startedAt, serial)
  }, ONLINE_STALL_RECOVERY_DELAY_MS)
}

function sameMusicTrackIdentity(left: MusicTrack | null | undefined, right: MusicTrack | null | undefined) {
  if (!left || !right || left.source !== right.source) {
    return false
  }

  if (left.source === 'netease' || right.source === 'netease') {
    return left.neteaseSongId !== undefined && left.neteaseSongId === right.neteaseSongId
  }

  if (left.source === 'kugou' || right.source === 'kugou') {
    return Boolean(left.kugouSongHash) && left.kugouSongHash === right.kugouSongHash
  }

  return left.id === right.id
}

function waitForOnlinePlaybackProgress(
  track: MusicTrack,
  resumeTime: number,
  options: { requireAdvance?: boolean } = {},
) {
  return new Promise<void>((resolve, reject) => {
    const element = audio.value
    if (!element) {
      reject(new Error('音频播放器不可用。'))
      return
    }

    const player = element
    let settled = false
    let timer: number | null = null
    const requireAdvance = options.requireAdvance ?? true
    const durationLimit = Number.isFinite(player.duration) && player.duration > 0
      ? Math.max(resumeTime, player.duration - 0.25)
      : resumeTime + ONLINE_STALL_RECOVERY_PROGRESS_SECONDS
    const targetTime = Math.min(
      resumeTime + ONLINE_STALL_RECOVERY_PROGRESS_SECONDS,
      durationLimit,
    )
    const cleanup = () => {
      player.removeEventListener('timeupdate', checkProgress)
      player.removeEventListener('playing', checkProgress)
      player.removeEventListener('canplay', checkProgress)
      player.removeEventListener('progress', checkProgress)
      player.removeEventListener('error', handleError)
      if (timer !== null) {
        window.clearTimeout(timer)
        timer = null
      }
    }
    const settle = (callback: () => void) => {
      if (settled) {
        return
      }
      settled = true
      cleanup()
      callback()
    }
    const handleError = () => {
      settle(() => reject(new Error('音频元素读取刷新后的播放链路失败。')))
    }
    function checkProgress() {
      if (!sameMusicTrackIdentity(currentTrack.value, track)) {
        settle(() => reject(new Error('播放目标已切换。')))
        return
      }

      const time = player.currentTime || currentTime.value
      if (
        !requireAdvance &&
        !player.paused &&
        player.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA &&
        time >= Math.max(resumeTime - 0.25, 0)
      ) {
        settle(() => resolve())
        return
      }

      if (player.ended || (!player.paused && time > targetTime)) {
        settle(() => resolve())
      }
    }

    player.addEventListener('timeupdate', checkProgress)
    player.addEventListener('playing', checkProgress)
    player.addEventListener('canplay', checkProgress)
    player.addEventListener('progress', checkProgress)
    player.addEventListener('error', handleError)
    timer = window.setTimeout(() => {
      settle(() => reject(new Error('刷新播放链路后仍未检测到播放进度推进。')))
    }, ONLINE_STALL_RECOVERY_VERIFY_MS)
    checkProgress()
  })
}

function scheduleOnlineStallRecoveryRetry(stalledTrack: MusicTrack, reason: string) {
  if (!audio.value || !sameMusicTrackIdentity(currentTrack.value, stalledTrack) || !playing.value) {
    return
  }

  clearOnlineStallRecoveryTimer()
  const startedAt = audio.value.currentTime || currentTime.value || onlineStallStartedAt
  onlineStallStartedAt = startedAt
  const serial = ++onlineStallRecoverySerial
  playerStatus.value =
    `当前${currentTrackPlatformLabel.value}恢复失败：${reason}。正在重新尝试刷新播放链路...`
  onlineStallRecoveryTimer = window.setTimeout(() => {
    onlineStallRecoveryTimer = null
    void recoverOnlinePlaybackStall(stalledTrack, startedAt, serial)
  }, ONLINE_STALL_RECOVERY_RETRY_DELAY_MS)
}

async function skipCurrentOnlineTrackAfterStall(stalledTrack: MusicTrack, reason: string) {
  const sourceTrack = platformTrackForCurrentPlayback(stalledTrack)
  if (!sourceTrack) {
    playerStatus.value = ''
    playerError.value = `当前${currentTrackPlatformLabel.value}持续无数据：${reason}。没有找到可自动跳过的源列表。`
    return
  }

  if (sourceTrack.source === 'netease') {
    const sourceTracks = currentNeteasePlaybackTracks()
    markOnlineTrackUnavailable(neteaseUnavailableTracks, sourceTrack.track.id, reason)
    await skipUnavailableNeteaseTrack(sourceTrack.track, sourceTracks, reason)
    return
  }

  const sourceTracks = currentKugouPlaybackTracks()
  const key = kugouPlaybackCacheKey(sourceTrack.track)
  if (key) {
    markOnlineTrackUnavailable(kugouUnavailableTracks, key, reason)
  }
  await skipUnavailableKugouTrack(sourceTrack.track, sourceTracks, reason)
}

async function recoverOnlinePlaybackStall(
  stalledTrack: MusicTrack,
  stalledAt: number,
  serial: number,
) {
  if (
    serial !== onlineStallRecoverySerial ||
    !sameMusicTrackIdentity(currentTrack.value, stalledTrack) ||
    !audio.value ||
    !playing.value ||
    !currentTrackOnline.value
  ) {
    return
  }

  const currentAudioTime = audio.value.currentTime || currentTime.value
  const recoveredNaturally =
    currentAudioTime > stalledAt + ONLINE_STALL_RECOVERY_PROGRESS_SECONDS
  if (recoveredNaturally) {
    resetOnlineStallRecovery()
    return
  }

  if (onlineStallRecoveryAttempts >= MAX_ONLINE_STALL_RECOVERY_ATTEMPTS) {
    await skipCurrentOnlineTrackAfterStall(
      stalledTrack,
      '连续多次长时间无数据，刷新播放链路后仍未恢复。',
    )
    return
  }

  onlineStallRecoveryAttempts += 1
  const resumeTime = Math.max(currentAudioTime, onlineStallStartedAt, 0)
  playerStatus.value = `当前${currentTrackPlatformLabel.value}长时间无数据，正在刷新播放链路并尝试从 ${formatTime(resumeTime)} 继续...`

  try {
    await refreshCurrentOnlinePlaybackAt(resumeTime, stalledTrack)
  } catch (err) {
    if (sameMusicTrackIdentity(currentTrack.value, stalledTrack)) {
      const reason = normalizeOnlineStallRecoveryFailureReason(err)
      if (!playing.value && playerError.value) {
        await skipCurrentOnlineTrackAfterStall(stalledTrack, reason)
        return
      }
      if (onlineStallRecoveryAttempts >= MAX_ONLINE_STALL_RECOVERY_ATTEMPTS) {
        await skipCurrentOnlineTrackAfterStall(stalledTrack, reason)
        return
      }
      scheduleOnlineStallRecoveryRetry(stalledTrack, reason)
    }
  }
}

async function refreshCurrentOnlinePlaybackAt(
  resumeTime: number,
  expectedTrack: MusicTrack,
  options: { reason?: 'stall' | 'quality-switch' } = {},
) {
  const sourceTrack = platformTrackForCurrentPlayback(expectedTrack)
  if (!sourceTrack) {
    throw new Error('没有找到当前在线歌曲的源列表信息。')
  }

  invalidateCurrentOnlinePlaybackCache()
  let refreshedTrack: MusicTrack
  if (sourceTrack.source === 'netease') {
    const playback = await fetchNeteasePlayback(sourceTrack.track)
    cacheNeteasePlayback(sourceTrack.track, playback)
    refreshedTrack = createNeteaseMusicTrack(sourceTrack.track, playback)
  } else {
    const playback = await fetchKugouPlayback(sourceTrack.track)
    cacheKugouPlayback(sourceTrack.track, playback)
    refreshedTrack = createKugouMusicTrack(sourceTrack.track, playback)
  }

  if (!sameMusicTrackIdentity(currentTrack.value, expectedTrack)) {
    return
  }

  neteaseCurrentTrack.value = refreshedTrack
  currentTime.value = resumeTime
  visualPlaybackTime.value = resumeTime
  await nextTick()

  if (!audio.value || !sameMusicTrackIdentity(currentTrack.value, refreshedTrack)) {
    throw new Error('新音质播放源已获取，但当前播放目标已变化。')
  }

  const resumed = await loadAndPlayCurrentAudioAt(refreshedTrack, resumeTime)
  if (!resumed) {
    throw new Error(playerError.value || '刷新播放链路后未能继续播放。')
  }
  await waitForOnlinePlaybackProgress(refreshedTrack, resumeTime, {
    requireAdvance: options.reason !== 'quality-switch',
  })
  if (!playerError.value) {
    if (options.reason === 'quality-switch') {
      const qualityText = formatTrackPlaybackQuality(refreshedTrack).replace(/^音质：/, '')
      playerStatus.value = qualityText
        ? `已切换在线播放音质，实际播放：${qualityText}。`
        : `已切换在线播放音质，继续播放《${refreshedTrack.title}》。`
    } else {
      playerStatus.value = `已刷新${currentTrackPlatformLabel.value}播放链路，继续播放《${refreshedTrack.title}》。`
    }
    scheduleOnlinePlaybackPrefetch()
  }
}

function platformTrackForCurrentPlayback(track: MusicTrack):
  | { source: 'netease'; track: NeteasePlaylistTrack }
  | { source: 'kugou'; track: KugouSearchTrack }
  | null {
  if (track.source === 'netease' && track.neteaseSongId) {
    const sourceTrack = currentNeteasePlaybackTracks().find(
      (candidate) => candidate.id === track.neteaseSongId,
    )
    return sourceTrack ? { source: 'netease', track: sourceTrack } : null
  }

  if (track.source === 'kugou' && track.kugouSongHash) {
    const sourceTrack = currentKugouPlaybackTracks().find(
      (candidate) => candidate.hash === track.kugouSongHash,
    )
    return sourceTrack ? { source: 'kugou', track: sourceTrack } : null
  }

  return null
}

async function handleEnded() {
  if (repeatMode.value === 'one') {
    await playCurrent(true)
    return
  }

  await playNext(false)
}

async function updateKugouProxyErrorDetail(track: MusicTrack) {
  if (!track.url) {
    return
  }

  try {
    const status = await invoke<KugouPlaybackProxyStatus>('get_kugou_playback_proxy_status', {
      proxyUrl: track.url,
    })
    if (!sameMusicTrackIdentity(currentTrack.value, track)) {
      return
    }

    const detailParts = [
      formatKugouProxyStatusMessage(status.message),
      status.lastRange ? `失败位置：${status.lastRange}` : '',
      status.refreshCount > 0 ? `已刷新 ${status.refreshCount} 次` : '',
    ].filter(Boolean)
    playerError.value = detailParts.length > 0
      ? `当前酷狗在线音乐无法继续读取。${detailParts.join('；')}`
      : '当前酷狗在线音乐无法继续读取，代理没有返回更多诊断信息。'
  } catch (err) {
    if (!sameMusicTrackIdentity(currentTrack.value, track)) {
      return
    }
    playerError.value =
      `当前酷狗在线音乐无法继续读取，且代理诊断读取失败：${String(err)}`
  }
}

function formatKugouProxyStatusMessage(message: string) {
  if (message.includes('音频流读取中断') || message.includes('本机播放器音频流失败')) {
    return `${message}。这通常是 CDN 网络波动、临时链接中断或播放器拖动进度时旧 Range 被取消导致。`
  }

  return message
}

function handleAudioError() {
  playing.value = false
  playerStatus.value = ''
  invalidateCurrentOnlinePlaybackCache()
  if (currentTrack.value?.source === 'kugou') {
    const track = currentTrack.value
    playerError.value =
      '当前酷狗在线音乐无法通过本机播放代理继续读取，正在读取代理诊断...'
    void updateKugouProxyErrorDetail(track)
    return
  }

  playerError.value = currentTrackOnline.value
    ? `当前${currentTrackPlatformLabel.value}无法读取，播放链接可能已过期或受版权、会员、地区限制。`
    : '当前音频无法读取，请确认文件仍在原位置并且格式受系统支持。'
}

async function prepareImmersiveVisualization() {
  if (audio.value && playing.value) {
    startVisualClock()
    connectAudioElement(audio.value)
    void resumeAnalyzer()
  }

  scheduleImmersiveContentPreparation()
}

function clearImmersiveContentPrepTimer() {
  immersiveContentPrepSerial += 1
  if (immersiveContentPrepTimer !== null) {
    window.clearTimeout(immersiveContentPrepTimer)
    immersiveContentPrepTimer = null
  }
}

function scheduleImmersiveContentPreparation(delayMs = IMMERSIVE_CONTENT_PREP_DELAY_MS) {
  clearImmersiveContentPrepTimer()
  const track = currentTrack.value
  if (!immersiveMode.value || !track) {
    return
  }

  const serial = immersiveContentPrepSerial
  const trackId = track.id
  immersiveContentPrepTimer = window.setTimeout(() => {
    immersiveContentPrepTimer = null
    if (
      serial !== immersiveContentPrepSerial ||
      !immersiveMode.value ||
      currentTrack.value?.id !== trackId
    ) {
      return
    }

    void prepareImmersiveContent()
  }, delayMs)
}

async function prepareImmersiveContent() {
  const track = currentTrack.value
  if (!track || !immersiveMode.value) {
    return
  }

  void analyzeCurrentTrackBeatMap()
  void loadCurrentTrackLyrics()
}

function startVisualClock() {
  if (visualClockFrameId !== null) {
    return
  }

  lastVisualClockUpdate = 0
  lastLyricMusicEnvelopeUpdate = 0

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
    updateLyricMusicEnvelope(timestamp)
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

function resetLyricMusicEnvelope() {
  lyricMusicEnvelope.value = { ...LYRIC_MUSIC_ENVELOPE_ZERO }
  lastLyricMusicEnvelopeUpdate = 0
}

function updateLyricMusicEnvelope(timestamp: number) {
  if (!immersiveMode.value || !playing.value || !currentTrack.value || visualReducedMotion.value) {
    resetLyricMusicEnvelope()
    return
  }

  const deltaSeconds = lastLyricMusicEnvelopeUpdate
    ? clamp((timestamp - lastLyricMusicEnvelopeUpdate) / 1000, 1 / 90, 0.16)
    : 1 / 30
  lastLyricMusicEnvelopeUpdate = timestamp

  const energy = visualEnergyFrame.value
  const lyric = immersiveLyrics.value
  const progress = clamp(lyric.progress, 0, 1)
  const lyricActive = lyric.status === 'ready' && !lyric.interlude
  const vocalArc = lyricActive ? Math.sin(Math.PI * progress) : 0
  const phraseTarget = clamp(vocalArc * 0.42, 0, 1)
  const pulseTarget = 0
  const breathTarget = clamp(energy.volume * 0.54 + energy.mid * 0.28 + vocalArc * 0.18, 0, 1)
  const airTarget = clamp(energy.treble * 0.48 + energy.mid * 0.28 + vocalArc * 0.14, 0, 1)
  const focusTarget = clamp(0.22 + phraseTarget * 0.44 + pulseTarget * 0.18 + breathTarget * 0.16, 0, 1)
  const driftTarget =
    Math.sin(visualTimeValue.value * 0.48 + energy.mid * 1.4) *
    clamp(0.24 + breathTarget * 0.5 + airTarget * 0.26, 0, 1)
  const current = lyricMusicEnvelope.value

  lyricMusicEnvelope.value = {
    pulse: smoothEnvelopeValue(current.pulse, pulseTarget, 0.045, 0.34, deltaSeconds),
    breath: smoothEnvelopeValue(current.breath, breathTarget, 0.3, 0.62, deltaSeconds),
    phrase: smoothEnvelopeValue(current.phrase, phraseTarget, 0.12, 0.42, deltaSeconds),
    air: smoothEnvelopeValue(current.air, airTarget, 0.18, 0.5, deltaSeconds),
    drift: smoothEnvelopeValue(current.drift, driftTarget, 0.34, 0.42, deltaSeconds),
    focus: smoothEnvelopeValue(current.focus, focusTarget, 0.12, 0.36, deltaSeconds),
  }
}

async function analyzeCurrentTrackBeatMap() {
  const track = currentTrack.value
  if (!track) {
    return
  }

  if (track.source === 'netease' || track.source === 'kugou' || playing.value) {
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
    coverImgUrl: normalizeCoverImgUrl(metadata.coverImgUrl) ?? '',
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
  if (candidate.coverImgUrl) {
    track.coverImgUrl = candidate.coverImgUrl
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

async function playCustomPlaylistTrack(playlist: MusicPlaylist, trackId: string) {
  const trackIds = normalizePlaylistTrackIds(playlist.trackIds)
  if (!trackIds.includes(trackId)) {
    playerError.value = '这个歌单里找不到这首歌曲。'
    return
  }

  const index = tracks.value.findIndex((track) => track.id === trackId)
  if (index < 0) {
    playerError.value = '这首歌曲已经不在本地音乐库中。'
    return
  }

  await playTrack(index, trackIds)
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

function isImmersiveFreeCameraInUse() {
  return immersiveFreeCameraActive.value || immersiveFreeCameraLocked.value || immersiveFreeCameraResetting.value
}

function isEditableEventTarget(target: EventTarget | null) {
  return target instanceof HTMLElement
    ? Boolean(target.closest('input, textarea, select, [contenteditable="true"]'))
    : false
}

function handleImmersiveFreeCameraKeyDown(event: KeyboardEvent) {
  consumeImmersiveFreeCameraKey(event, true)
}

function handleImmersiveFreeCameraKeyUp(event: KeyboardEvent) {
  consumeImmersiveFreeCameraKey(event, false)
}

function consumeImmersiveFreeCameraKey(event: KeyboardEvent, isDown: boolean) {
  if (!immersiveMode.value || event.altKey || event.metaKey) {
    return false
  }

  if (isDown && event.code === 'Escape' && immersiveStageOnlyMode.value) {
    event.preventDefault()
    event.stopImmediatePropagation()
    setImmersiveStageOnlyMode(false)
    return true
  }

  if (isEditableEventTarget(event.target)) {
    return false
  }

  if (isDown && event.code === 'KeyR' && !event.ctrlKey) {
    event.preventDefault()
    event.stopImmediatePropagation()
    if (!event.repeat) {
      toggleImmersiveFreeCamera()
    }
    return true
  }

  if (isDown && event.code === 'KeyK' && !event.ctrlKey && isImmersiveFreeCameraInUse()) {
    event.preventDefault()
    event.stopImmediatePropagation()
    resetImmersiveFreeCamera()
    return true
  }

  if (!immersiveFreeCameraActive.value || !IMMERSIVE_FREE_CAMERA_CONTROL_CODES.has(event.code)) {
    return false
  }

  event.preventDefault()
  event.stopImmediatePropagation()

  if (isDown) {
    immersiveFreeCameraKeys.add(event.code)
  } else {
    immersiveFreeCameraKeys.delete(event.code)
  }

  scheduleImmersiveFreeCameraFrame()
  return true
}

function handleImmersiveFreeCameraMouseMove(event: MouseEvent) {
  if (!immersiveMode.value || !immersiveFreeCameraActive.value) {
    immersiveFreeCameraPointerSeen = false
    return
  }

  let movementX = event.movementX || 0
  let movementY = event.movementY || 0

  if ((!movementX && !movementY) && immersiveFreeCameraPointerSeen) {
    movementX = event.clientX - immersiveFreeCameraPointerX
    movementY = event.clientY - immersiveFreeCameraPointerY
  }

  immersiveFreeCameraPointerX = event.clientX
  immersiveFreeCameraPointerY = event.clientY
  immersiveFreeCameraPointerSeen = true

  if (!movementX && !movementY) {
    return
  }

  const motionScale = visualReducedMotion.value ? 0.42 : 1
  immersiveFreeCameraYaw.value -= movementX * IMMERSIVE_FREE_CAMERA_MOUSE_FACTOR * motionScale
  immersiveFreeCameraPitch.value = clamp(
    immersiveFreeCameraPitch.value - movementY * IMMERSIVE_FREE_CAMERA_MOUSE_FACTOR * motionScale,
    -Math.PI * 0.49,
    Math.PI * 0.49,
  )
}

function handleImmersiveSceneWheel(event: WheelEvent) {
  if (!isImmersiveFreeCameraInUse()) {
    return
  }

  event.preventDefault()
  immersiveFreeCameraFov.value = clamp(immersiveFreeCameraFov.value + event.deltaY * 0.018, 28, 72)
}

function toggleImmersiveFreeCamera() {
  if (!immersiveMode.value || !webglStarfieldActive.value) {
    return
  }

  clearImmersiveStageMomentum()
  immersiveStageDragging.value = false
  immersiveStagePointerId = null
  immersiveFreeCameraResetting.value = false
  immersiveFreeCameraKeys.clear()
  immersiveFreeCameraVelocityX = 0
  immersiveFreeCameraVelocityY = 0
  immersiveFreeCameraVelocityZ = 0

  if (immersiveFreeCameraActive.value) {
    immersiveFreeCameraActive.value = false
    immersiveFreeCameraLocked.value = true
    immersiveFreeCameraPointerSeen = false
    exitImmersiveFreeCameraPointerLock()
    clearImmersiveFreeCameraFrame()
    return
  }

  if (!immersiveFreeCameraLocked.value) {
    captureImmersiveFreeCameraFromStage()
  }

  immersiveFreeCameraActive.value = true
  immersiveFreeCameraLocked.value = true
  immersiveFreeCameraPointerSeen = false
  requestImmersiveFreeCameraPointerLock()
  scheduleImmersiveFreeCameraFrame()
}

function captureImmersiveFreeCameraFromStage() {
  const radians = Math.PI / 180
  immersiveFreeCameraX.value = IMMERSIVE_FREE_CAMERA_DEFAULT.x
  immersiveFreeCameraY.value = IMMERSIVE_FREE_CAMERA_DEFAULT.y
  immersiveFreeCameraZ.value = IMMERSIVE_FREE_CAMERA_DEFAULT.z
  immersiveFreeCameraYaw.value = immersiveStageYaw.value * radians * 1.18
  immersiveFreeCameraPitch.value = clamp(
    IMMERSIVE_FREE_CAMERA_DEFAULT.pitch + immersiveStagePitch.value * radians * 0.96,
    -Math.PI * 0.49,
    Math.PI * 0.49,
  )
  immersiveFreeCameraRoll.value = IMMERSIVE_FREE_CAMERA_DEFAULT.roll
  immersiveFreeCameraFov.value = IMMERSIVE_FREE_CAMERA_DEFAULT.fov
}

function requestImmersiveFreeCameraPointerLock() {
  const scene = immersiveScene.value
  if (!scene?.requestPointerLock) {
    return
  }

  try {
    const result = scene.requestPointerLock()
    if (result && typeof (result as Promise<void>).catch === 'function') {
      ;(result as Promise<void>).catch(() => {
        immersiveFreeCameraPointerSeen = false
      })
    }
  } catch {
    immersiveFreeCameraPointerSeen = false
  }
}

function exitImmersiveFreeCameraPointerLock() {
  try {
    if (document.pointerLockElement === immersiveScene.value) {
      document.exitPointerLock()
    }
  } catch {
    // Pointer lock is optional; failing to release it should not break playback controls.
  }
}

function resetImmersiveFreeCamera() {
  if (!isImmersiveFreeCameraInUse()) {
    resetImmersiveStageView()
    return
  }

  immersiveFreeCameraResetFrom = {
    x: immersiveFreeCameraX.value,
    y: immersiveFreeCameraY.value,
    z: immersiveFreeCameraZ.value,
    yaw: immersiveFreeCameraYaw.value,
    pitch: immersiveFreeCameraPitch.value,
    roll: immersiveFreeCameraRoll.value,
    fov: immersiveFreeCameraFov.value,
  }
  immersiveFreeCameraResetStart = performance.now()
  immersiveFreeCameraActive.value = false
  immersiveFreeCameraLocked.value = true
  immersiveFreeCameraResetting.value = true
  immersiveFreeCameraKeys.clear()
  immersiveFreeCameraVelocityX = 0
  immersiveFreeCameraVelocityY = 0
  immersiveFreeCameraVelocityZ = 0
  immersiveFreeCameraPointerSeen = false
  exitImmersiveFreeCameraPointerLock()
  scheduleImmersiveFreeCameraFrame()
  resetImmersiveStageView(undefined, true)
}

function clearImmersiveFreeCamera() {
  immersiveFreeCameraActive.value = false
  immersiveFreeCameraLocked.value = false
  immersiveFreeCameraResetting.value = false
  immersiveFreeCameraKeys.clear()
  immersiveFreeCameraVelocityX = 0
  immersiveFreeCameraVelocityY = 0
  immersiveFreeCameraVelocityZ = 0
  immersiveFreeCameraPointerSeen = false
  setImmersiveFreeCameraDefaultPose()
  exitImmersiveFreeCameraPointerLock()
  clearImmersiveFreeCameraFrame()
}

function setImmersiveFreeCameraDefaultPose() {
  immersiveFreeCameraX.value = IMMERSIVE_FREE_CAMERA_DEFAULT.x
  immersiveFreeCameraY.value = IMMERSIVE_FREE_CAMERA_DEFAULT.y
  immersiveFreeCameraZ.value = IMMERSIVE_FREE_CAMERA_DEFAULT.z
  immersiveFreeCameraYaw.value = IMMERSIVE_FREE_CAMERA_DEFAULT.yaw
  immersiveFreeCameraPitch.value = IMMERSIVE_FREE_CAMERA_DEFAULT.pitch
  immersiveFreeCameraRoll.value = IMMERSIVE_FREE_CAMERA_DEFAULT.roll
  immersiveFreeCameraFov.value = IMMERSIVE_FREE_CAMERA_DEFAULT.fov
}

function scheduleImmersiveFreeCameraFrame() {
  if (immersiveFreeCameraFrameId !== null) {
    return
  }

  immersiveFreeCameraFrameId = window.requestAnimationFrame(stepImmersiveFreeCameraFrame)
}

function clearImmersiveFreeCameraFrame() {
  if (immersiveFreeCameraFrameId !== null) {
    window.cancelAnimationFrame(immersiveFreeCameraFrameId)
    immersiveFreeCameraFrameId = null
  }
  immersiveFreeCameraLastFrame = 0
}

function stepImmersiveFreeCameraFrame(timestamp: number) {
  immersiveFreeCameraFrameId = null
  const deltaSeconds = immersiveFreeCameraLastFrame
    ? clamp((timestamp - immersiveFreeCameraLastFrame) / 1000, 1 / 120, 0.08)
    : 1 / 60
  immersiveFreeCameraLastFrame = timestamp

  if (immersiveFreeCameraResetting.value) {
    updateImmersiveFreeCameraReset(timestamp)
  } else if (immersiveFreeCameraActive.value) {
    updateImmersiveFreeCameraMotion(deltaSeconds)
  }

  if (immersiveFreeCameraResetting.value || immersiveFreeCameraActive.value) {
    scheduleImmersiveFreeCameraFrame()
  } else {
    immersiveFreeCameraLastFrame = 0
  }
}

function updateImmersiveFreeCameraReset(timestamp: number) {
  const progress = clamp((timestamp - immersiveFreeCameraResetStart) / IMMERSIVE_FREE_CAMERA_RESET_MS, 0, 1)
  const eased = 1 - Math.pow(1 - progress, 3)

  immersiveFreeCameraX.value = lerpNumber(immersiveFreeCameraResetFrom.x, IMMERSIVE_FREE_CAMERA_DEFAULT.x, eased)
  immersiveFreeCameraY.value = lerpNumber(immersiveFreeCameraResetFrom.y, IMMERSIVE_FREE_CAMERA_DEFAULT.y, eased)
  immersiveFreeCameraZ.value = lerpNumber(immersiveFreeCameraResetFrom.z, IMMERSIVE_FREE_CAMERA_DEFAULT.z, eased)
  immersiveFreeCameraYaw.value = lerpNumber(immersiveFreeCameraResetFrom.yaw, IMMERSIVE_FREE_CAMERA_DEFAULT.yaw, eased)
  immersiveFreeCameraPitch.value = lerpNumber(immersiveFreeCameraResetFrom.pitch, IMMERSIVE_FREE_CAMERA_DEFAULT.pitch, eased)
  immersiveFreeCameraRoll.value = lerpNumber(immersiveFreeCameraResetFrom.roll, IMMERSIVE_FREE_CAMERA_DEFAULT.roll, eased)
  immersiveFreeCameraFov.value = lerpNumber(immersiveFreeCameraResetFrom.fov, IMMERSIVE_FREE_CAMERA_DEFAULT.fov, eased)

  if (progress >= 1) {
    setImmersiveFreeCameraDefaultPose()
    immersiveFreeCameraResetting.value = false
    immersiveFreeCameraLocked.value = false
  }
}

function updateImmersiveFreeCameraMotion(deltaSeconds: number) {
  const forwardIntent =
    (immersiveFreeCameraKeys.has('KeyW') ? 1 : 0) - (immersiveFreeCameraKeys.has('KeyS') ? 1 : 0)
  const sideIntent =
    (immersiveFreeCameraKeys.has('KeyD') ? 1 : 0) - (immersiveFreeCameraKeys.has('KeyA') ? 1 : 0)
  const liftIntent =
    (immersiveFreeCameraKeys.has('Space') ? 1 : 0) -
    (immersiveFreeCameraKeys.has('ControlLeft') || immersiveFreeCameraKeys.has('ControlRight') ? 1 : 0)
  const yaw = immersiveFreeCameraYaw.value
  const pitch = immersiveFreeCameraPitch.value
  const forwardX = -Math.sin(yaw) * Math.cos(pitch)
  const forwardY = Math.sin(pitch)
  const forwardZ = -Math.cos(yaw) * Math.cos(pitch)
  const rightX = Math.cos(yaw)
  const rightZ = -Math.sin(yaw)
  let targetX = rightX * sideIntent + forwardX * forwardIntent
  let targetY = liftIntent + forwardY * forwardIntent
  let targetZ = rightZ * sideIntent + forwardZ * forwardIntent
  const targetLength = Math.hypot(targetX, targetY, targetZ)

  if (targetLength > 1) {
    targetX /= targetLength
    targetY /= targetLength
    targetZ /= targetLength
  }

  const fast = immersiveFreeCameraKeys.has('ShiftLeft') || immersiveFreeCameraKeys.has('ShiftRight')
  const speed = fast ? IMMERSIVE_FREE_CAMERA_FAST_SPEED : IMMERSIVE_FREE_CAMERA_BASE_SPEED
  targetX *= speed
  targetY *= speed
  targetZ *= speed

  const easing = targetLength > 0 ? 8.2 : 13.5
  const mix = clamp(easing * deltaSeconds, 0, 1)
  immersiveFreeCameraVelocityX = lerpNumber(immersiveFreeCameraVelocityX, targetX, mix)
  immersiveFreeCameraVelocityY = lerpNumber(immersiveFreeCameraVelocityY, targetY, mix)
  immersiveFreeCameraVelocityZ = lerpNumber(immersiveFreeCameraVelocityZ, targetZ, mix)

  if (Math.hypot(immersiveFreeCameraVelocityX, immersiveFreeCameraVelocityY, immersiveFreeCameraVelocityZ) < 0.02) {
    immersiveFreeCameraVelocityX = 0
    immersiveFreeCameraVelocityY = 0
    immersiveFreeCameraVelocityZ = 0
  }

  immersiveFreeCameraX.value = clamp(immersiveFreeCameraX.value + immersiveFreeCameraVelocityX * deltaSeconds, -14, 14)
  immersiveFreeCameraY.value = clamp(immersiveFreeCameraY.value + immersiveFreeCameraVelocityY * deltaSeconds, -7, 9)
  immersiveFreeCameraZ.value = clamp(immersiveFreeCameraZ.value + immersiveFreeCameraVelocityZ * deltaSeconds, -10, 18)

  const rollIntent =
    (immersiveFreeCameraKeys.has('KeyQ') ? 1 : 0) - (immersiveFreeCameraKeys.has('KeyE') ? 1 : 0)
  immersiveFreeCameraRoll.value = clamp(
    immersiveFreeCameraRoll.value + rollIntent * deltaSeconds * 0.9,
    -0.78,
    0.78,
  )
}

function lerpNumber(from: number, to: number, amount: number) {
  return from + (to - from) * amount
}

function startImmersiveStageDrag(event: PointerEvent) {
  if (event.button !== 0 || event.defaultPrevented) {
    return
  }

  if (immersiveFreeCameraActive.value) {
    requestImmersiveFreeCameraPointerLock()
    event.preventDefault()
    return
  }

  if (isImmersiveFreeCameraInUse()) {
    event.preventDefault()
    return
  }

  clearImmersiveStageMomentum()
  immersiveStageDragging.value = true
  immersiveStagePointerId = event.pointerId
  immersiveStageLastX = event.clientX
  immersiveStageLastY = event.clientY
  immersiveStageVelocityYaw.value = 0
  immersiveStageVelocityPitch.value = 0
  event.preventDefault()
  const target = event.currentTarget as HTMLElement | null
  if (target?.setPointerCapture) {
    target.setPointerCapture(event.pointerId)
  }
}

function moveImmersiveStageDrag(event: PointerEvent) {
  if (!immersiveStageDragging.value || immersiveStagePointerId !== event.pointerId) {
    return
  }

  const deltaX = event.clientX - immersiveStageLastX
  const deltaY = event.clientY - immersiveStageLastY
  const motionScale = visualReducedMotion.value ? 0.42 : 1
  const yawDelta = deltaX * IMMERSIVE_STAGE_DRAG_YAW_FACTOR * motionScale
  const pitchDelta = -deltaY * IMMERSIVE_STAGE_DRAG_PITCH_FACTOR * motionScale

  immersiveStageYaw.value = clamp(
    immersiveStageYaw.value + yawDelta,
    -IMMERSIVE_STAGE_MAX_YAW,
    IMMERSIVE_STAGE_MAX_YAW,
  )
  immersiveStagePitch.value = clamp(
    immersiveStagePitch.value + pitchDelta,
    -IMMERSIVE_STAGE_MAX_PITCH,
    IMMERSIVE_STAGE_MAX_PITCH,
  )
  immersiveStageVelocityYaw.value = yawDelta
  immersiveStageVelocityPitch.value = pitchDelta
  immersiveStageLastX = event.clientX
  immersiveStageLastY = event.clientY
}

function finishImmersiveStageDrag(event: PointerEvent) {
  if (immersiveStagePointerId !== event.pointerId) {
    return
  }

  const target = event.currentTarget as HTMLElement | null
  if (target?.releasePointerCapture && target.hasPointerCapture?.(event.pointerId)) {
    target.releasePointerCapture(event.pointerId)
  }
  immersiveStageDragging.value = false
  immersiveStagePointerId = null

  if (visualReducedMotion.value || visualStagePreset.value === 'galaxy') {
    immersiveStageVelocityYaw.value = 0
    immersiveStageVelocityPitch.value = 0
    return
  }

  startImmersiveStageMomentum()
}

function resetImmersiveStageView(event?: MouseEvent, skipFreeCamera = false) {
  event?.preventDefault()
  if (!skipFreeCamera && isImmersiveFreeCameraInUse()) {
    resetImmersiveFreeCamera()
    return
  }
  clearImmersiveStageMomentum()
  immersiveStageDragging.value = false
  immersiveStagePointerId = null
  immersiveStageYaw.value = 0
  immersiveStagePitch.value = 0
  immersiveStageVelocityYaw.value = 0
  immersiveStageVelocityPitch.value = 0
}

function startImmersiveStageMomentum() {
  clearImmersiveStageMomentum()
  const decay = 0.86

  const step = () => {
    immersiveStageVelocityYaw.value *= decay
    immersiveStageVelocityPitch.value *= decay

    if (
      Math.abs(immersiveStageVelocityYaw.value) < 0.015 &&
      Math.abs(immersiveStageVelocityPitch.value) < 0.015
    ) {
      immersiveStageVelocityYaw.value = 0
      immersiveStageVelocityPitch.value = 0
      immersiveStageMomentumFrameId = null
      return
    }

    immersiveStageYaw.value = clamp(
      immersiveStageYaw.value + immersiveStageVelocityYaw.value,
      -IMMERSIVE_STAGE_MAX_YAW,
      IMMERSIVE_STAGE_MAX_YAW,
    )
    immersiveStagePitch.value = clamp(
      immersiveStagePitch.value + immersiveStageVelocityPitch.value,
      -IMMERSIVE_STAGE_MAX_PITCH,
      IMMERSIVE_STAGE_MAX_PITCH,
    )
    immersiveStageMomentumFrameId = window.requestAnimationFrame(step)
  }

  immersiveStageMomentumFrameId = window.requestAnimationFrame(step)
}

function clearImmersiveStageMomentum() {
  if (immersiveStageMomentumFrameId !== null) {
    window.cancelAnimationFrame(immersiveStageMomentumFrameId)
    immersiveStageMomentumFrameId = null
  }
}

function clearMiniEdgeMoveTimer() {
  if (miniEdgeMoveTimer !== null) {
    window.clearTimeout(miniEdgeMoveTimer)
    miniEdgeMoveTimer = null
  }
}

function clearMiniEdgeHideTimer() {
  if (miniEdgeHideTimer !== null) {
    window.clearTimeout(miniEdgeHideTimer)
    miniEdgeHideTimer = null
  }
}

function clearMiniEdgeDragPollTimer() {
  if (miniEdgeDragPollTimer !== null) {
    window.clearTimeout(miniEdgeDragPollTimer)
    miniEdgeDragPollTimer = null
  }
}

function clearMiniEdgeTimers() {
  clearMiniEdgeMoveTimer()
  clearMiniEdgeHideTimer()
  clearMiniEdgeDragPollTimer()
}

function cancelMiniEdgeWindowMove() {
  miniEdgeWindowMoveSerial += 1
}

function waitMiniEdgeAnimationStep() {
  return new Promise<void>((resolve) => {
    window.setTimeout(resolve, MINI_EDGE_ANIMATION_STEP_MS)
  })
}

function workAreaFromMonitor(monitor: Monitor): ScreenWorkArea {
  const workArea = monitor.workArea
  return {
    left: workArea.position.x,
    top: workArea.position.y,
    right: workArea.position.x + workArea.size.width,
    bottom: workArea.position.y + workArea.size.height,
  }
}

function clampMiniEdgePosition(value: number, min: number, max: number) {
  return max < min ? min : clamp(value, min, max)
}

function easeMiniEdgeWindowMove(progress: number) {
  return 1 - (1 - progress) ** 3
}

async function setMiniEdgeWindowPosition(
  x: number,
  y: number,
  options: MiniEdgeWindowMoveOptions = {},
) {
  const targetX = Math.round(x)
  const targetY = Math.round(y)
  const moveSerial = miniEdgeWindowMoveSerial + 1
  miniEdgeWindowMoveSerial = moveSerial
  suppressMiniEdgeMoveUntil = Date.now() + MINI_EDGE_POSITION_SUPPRESS_MS

  const durationMs = options.animated ? Math.max(0, options.durationMs ?? MINI_EDGE_SNAP_ANIMATION_MS) : 0
  if (durationMs <= 0) {
    await musicWindow.setPosition(new PhysicalPosition(targetX, targetY))
    return
  }

  let startPosition: { x: number; y: number }
  try {
    startPosition = await musicWindow.outerPosition()
  } catch {
    await musicWindow.setPosition(new PhysicalPosition(targetX, targetY))
    return
  }

  const startX = startPosition.x
  const startY = startPosition.y
  const deltaX = targetX - startX
  const deltaY = targetY - startY
  if (Math.abs(deltaX) <= 1 && Math.abs(deltaY) <= 1) {
    await musicWindow.setPosition(new PhysicalPosition(targetX, targetY))
    return
  }

  const steps = Math.max(2, Math.ceil(durationMs / MINI_EDGE_ANIMATION_STEP_MS))
  for (let step = 1; step <= steps; step += 1) {
    if (moveSerial !== miniEdgeWindowMoveSerial) {
      return
    }

    const progress = easeMiniEdgeWindowMove(step / steps)
    const nextX = Math.round(startX + deltaX * progress)
    const nextY = Math.round(startY + deltaY * progress)
    suppressMiniEdgeMoveUntil = Date.now() + MINI_EDGE_POSITION_SUPPRESS_MS
    await musicWindow.setPosition(new PhysicalPosition(nextX, nextY))

    if (step < steps) {
      await waitMiniEdgeAnimationStep()
    }
  }
}

async function resolveMiniEdgeDockState(): Promise<MiniEdgeDockState | null> {
  const [position, size, activeMonitor] = await Promise.all([
    musicWindow.outerPosition(),
    musicWindow.outerSize(),
    currentMonitor().then((monitor) => monitor ?? primaryMonitor()),
  ])
  if (!activeMonitor) {
    return null
  }

  const scaleFactor = Number.isFinite(activeMonitor.scaleFactor) ? activeMonitor.scaleFactor : 1
  const snapDistance = MINI_EDGE_SNAP_DISTANCE * scaleFactor
  const visibleStrip = MINI_EDGE_VISIBLE_STRIP * scaleFactor
  const revealMargin = MINI_EDGE_REVEAL_MARGIN * scaleFactor
  const workArea = workAreaFromMonitor(activeMonitor)
  const candidates: Array<{ side: MiniEdgeDockSide; distance: number }> = [
    { side: 'left', distance: Math.abs(position.x - workArea.left) },
    { side: 'right', distance: Math.abs(workArea.right - (position.x + size.width)) },
    { side: 'top', distance: Math.abs(position.y - workArea.top) },
    { side: 'bottom', distance: Math.abs(workArea.bottom - (position.y + size.height)) },
  ]
  candidates.sort((a, b) => a.distance - b.distance)
  const nearest = candidates[0]

  if (!nearest || nearest.distance > snapDistance) {
    return null
  }

  const minX = workArea.left + revealMargin
  const maxX = workArea.right - size.width - revealMargin
  const minY = workArea.top + revealMargin
  const maxY = workArea.bottom - size.height - revealMargin
  const expandedX = clampMiniEdgePosition(position.x, minX, maxX)
  const expandedY = clampMiniEdgePosition(position.y, minY, maxY)

  if (nearest.side === 'left') {
    return {
      side: nearest.side,
      expandedX: workArea.left,
      expandedY,
      hiddenX: workArea.left - size.width + visibleStrip,
      hiddenY: expandedY,
    }
  }

  if (nearest.side === 'right') {
    return {
      side: nearest.side,
      expandedX: workArea.right - size.width,
      expandedY,
      hiddenX: workArea.right - visibleStrip,
      hiddenY: expandedY,
    }
  }

  if (nearest.side === 'top') {
    return {
      side: nearest.side,
      expandedX,
      expandedY: workArea.top,
      hiddenX: expandedX,
      hiddenY: workArea.top - size.height + visibleStrip,
    }
  }

  return {
    side: nearest.side,
    expandedX,
    expandedY: workArea.bottom - size.height,
    hiddenX: expandedX,
    hiddenY: workArea.bottom - visibleStrip,
  }
}

async function clearMiniEdgeDockState(restoreVisiblePosition = false) {
  miniEdgeInteractionSerial += 1
  clearMiniEdgeTimers()
  cancelMiniEdgeWindowMove()
  const previousState = miniEdgeDockState
  miniEdgeDockState = null
  miniEdgeDockSide.value = null
  miniEdgeDockExpanded.value = false

  if (restoreVisiblePosition && previousState) {
    try {
      await setMiniEdgeWindowPosition(previousState.expandedX, previousState.expandedY)
    } catch {
      // Restoring is best-effort when the OS is already changing the window mode.
    }
  }
}

function beginMiniEdgeManualDrag() {
  miniEdgeInteractionSerial += 1
  clearMiniEdgeTimers()
  cancelMiniEdgeWindowMove()
  miniEdgeDockState = null
  miniEdgeDockSide.value = null
  miniEdgeDockExpanded.value = false
  suppressMiniEdgeMoveUntil = 0
  return miniEdgeInteractionSerial
}

async function checkMiniEdgeDock(interactionSerial = miniEdgeInteractionSerial) {
  if (interactionSerial !== miniEdgeInteractionSerial) {
    return
  }

  if (!miniPlayerMode.value || immersiveMode.value || Date.now() < suppressMiniEdgeMoveUntil) {
    return
  }

  if (miniEdgeDockState && !miniEdgeDockExpanded.value) {
    return
  }

  let nextState: MiniEdgeDockState | null = null
  try {
    nextState = await resolveMiniEdgeDockState()
  } catch (err) {
    console.warn('音乐迷你窗口贴边判定失败', err)
    return
  }

  if (interactionSerial !== miniEdgeInteractionSerial || !miniPlayerMode.value || immersiveMode.value) {
    return
  }

  if (!nextState) {
    await clearMiniEdgeDockState(false)
    return
  }

  miniEdgeDockState = nextState
  miniEdgeDockSide.value = nextState.side
  miniEdgeDockExpanded.value = true

  try {
    await setMiniEdgeWindowPosition(nextState.expandedX, nextState.expandedY, {
      animated: true,
      durationMs: MINI_EDGE_SNAP_ANIMATION_MS,
    })
    if (interactionSerial !== miniEdgeInteractionSerial) {
      return
    }
    scheduleMiniEdgeAutoHide()
  } catch (err) {
    console.warn('音乐迷你窗口吸附失败', err)
    await clearMiniEdgeDockState(false)
  }
}

function scheduleMiniEdgeDockCheck(interactionSerial = miniEdgeInteractionSerial) {
  if (interactionSerial !== miniEdgeInteractionSerial) {
    return
  }

  if (!miniPlayerMode.value || immersiveMode.value || Date.now() < suppressMiniEdgeMoveUntil) {
    return
  }

  clearMiniEdgeMoveTimer()
  miniEdgeMoveTimer = window.setTimeout(() => {
    miniEdgeMoveTimer = null
    void checkMiniEdgeDock(interactionSerial)
  }, MINI_EDGE_MOVE_DEBOUNCE_MS)
}

function isMiniPlayerHovered() {
  return Boolean(miniPlayerElement.value?.matches(':hover'))
}

function scheduleMiniEdgeAutoHide() {
  if (!miniPlayerMode.value || immersiveMode.value || !miniEdgeDockState || !miniEdgeDockExpanded.value) {
    return
  }

  const interactionSerial = miniEdgeInteractionSerial
  clearMiniEdgeHideTimer()
  miniEdgeHideTimer = window.setTimeout(() => {
    miniEdgeHideTimer = null
    if (interactionSerial !== miniEdgeInteractionSerial) {
      return
    }

    if (!miniPlayerMode.value || immersiveMode.value || !miniEdgeDockState || !miniEdgeDockExpanded.value) {
      return
    }

    if (isMiniPlayerHovered()) {
      return
    }

    void hideMiniEdgeDock()
  }, MINI_EDGE_AUTO_HIDE_DELAY_MS)
}

async function pollMiniEdgeDragPosition(interactionSerial: number) {
  miniEdgeDragPollTimer = null
  if (interactionSerial !== miniEdgeInteractionSerial || !miniPlayerMode.value || immersiveMode.value) {
    return
  }

  let position: { x: number; y: number }
  try {
    position = await musicWindow.outerPosition()
  } catch {
    return
  }

  const lastPosition = miniEdgeDragPollLastPosition
  if (lastPosition) {
    const moved =
      Math.abs(position.x - lastPosition.x) > 1 ||
      Math.abs(position.y - lastPosition.y) > 1
    if (moved) {
      miniEdgeDragPollSeenMove = true
      miniEdgeDragPollIdleTicks = 0
      miniEdgeDragPollLastPosition = { x: position.x, y: position.y }
    } else if (miniEdgeDragPollSeenMove) {
      miniEdgeDragPollIdleTicks += 1
    }
  } else {
    miniEdgeDragPollLastPosition = { x: position.x, y: position.y }
  }

  const elapsed = Date.now() - miniEdgeDragPollStartedAt
  if (
    (miniEdgeDragPollSeenMove && miniEdgeDragPollIdleTicks >= MINI_EDGE_DRAG_POLL_IDLE_TICKS) ||
    elapsed >= MINI_EDGE_DRAG_POLL_MAX_MS
  ) {
    void checkMiniEdgeDock(interactionSerial)
    return
  }

  miniEdgeDragPollTimer = window.setTimeout(() => {
    void pollMiniEdgeDragPosition(interactionSerial)
  }, MINI_EDGE_DRAG_POLL_INTERVAL_MS)
}

function startMiniEdgeDragPositionPoll(interactionSerial: number) {
  if (!miniPlayerMode.value || immersiveMode.value) {
    return
  }

  clearMiniEdgeDragPollTimer()
  miniEdgeDragPollStartedAt = Date.now()
  miniEdgeDragPollLastPosition = null
  miniEdgeDragPollIdleTicks = 0
  miniEdgeDragPollSeenMove = false
  miniEdgeDragPollTimer = window.setTimeout(() => {
    void pollMiniEdgeDragPosition(interactionSerial)
  }, MINI_EDGE_DRAG_POLL_INTERVAL_MS)
}

function scheduleMiniEdgeFallbackChecks(interactionSerial: number) {
  window.setTimeout(() => {
    void checkMiniEdgeDock(interactionSerial)
  }, 700)
  window.setTimeout(() => {
    void checkMiniEdgeDock(interactionSerial)
  }, 1500)
  window.setTimeout(() => {
    void checkMiniEdgeDock(interactionSerial)
  }, 2600)
}

async function revealMiniEdgeDock() {
  clearMiniEdgeHideTimer()
  if (!miniPlayerMode.value || immersiveMode.value || !miniEdgeDockState || miniEdgeDockExpanded.value) {
    return
  }

  miniEdgeDockExpanded.value = true
  try {
    await setMiniEdgeWindowPosition(miniEdgeDockState.expandedX, miniEdgeDockState.expandedY, {
      animated: true,
      durationMs: MINI_EDGE_REVEAL_ANIMATION_MS,
    })
  } catch {
    miniEdgeDockExpanded.value = false
  }
}

async function hideMiniEdgeDock() {
  if (!miniPlayerMode.value || immersiveMode.value || !miniEdgeDockState || !miniEdgeDockExpanded.value) {
    return
  }

  miniEdgeDockExpanded.value = false
  try {
    await setMiniEdgeWindowPosition(miniEdgeDockState.hiddenX, miniEdgeDockState.hiddenY, {
      animated: true,
      durationMs: MINI_EDGE_HIDE_ANIMATION_MS,
    })
  } catch {
    await clearMiniEdgeDockState(false)
  }
}

function scheduleMiniEdgeRehide() {
  if (!miniPlayerMode.value || immersiveMode.value || !miniEdgeDockState || !miniEdgeDockExpanded.value) {
    return
  }

  clearMiniEdgeHideTimer()
  miniEdgeHideTimer = window.setTimeout(() => {
    miniEdgeHideTimer = null
    void hideMiniEdgeDock()
  }, MINI_EDGE_REHIDE_DELAY_MS)
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

  let dragInteractionSerial = miniEdgeInteractionSerial
  if (miniPlayerMode.value) {
    dragInteractionSerial = beginMiniEdgeManualDrag()
    startMiniEdgeDragPositionPoll(dragInteractionSerial)
  }

  try {
    await musicWindow.startDragging()
    if (miniPlayerMode.value) {
      scheduleMiniEdgeDockCheck(dragInteractionSerial)
      scheduleMiniEdgeFallbackChecks(dragInteractionSerial)
    }
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
    await clearMiniEdgeDockState(true)
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
    immersiveSearchInput.value?.blur()
    immersiveSearchFocused.value = false
    immersiveStageOnlyMode.value = false
    clearImmersiveContentPrepTimer()
    clearImmersiveFreeCamera()
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

  if (!nextMiniMode) {
    await clearMiniEdgeDockState(true)
  } else {
    await clearMiniEdgeDockState(false)
  }

  miniPlayerMode.value = nextMiniMode
  if (nextMiniMode) {
    immersiveMode.value = false
    immersiveStageOnlyMode.value = false
    clearImmersiveContentPrepTimer()
    clearImmersiveFreeCamera()
    resetImmersiveStageView()
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
    if (mode !== 'mini') {
      await clearMiniEdgeDockState(true)
    }

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
  if (miniPlayerMode.value) {
    await clearMiniEdgeDockState(true)
  }

  if (immersiveMode.value) {
    immersiveMode.value = false
    clearImmersiveContentPrepTimer()
    clearImmersiveFreeCamera()
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

function normalizeKugouQrStatus(result: KugouQrCheckResult) {
  if (result.loggedIn || result.status === 'authorized' || result.code === 4) {
    return 'authorized'
  }

  if (result.status === 'scanned' || result.code === 2 || result.code === 3) {
    return 'scanned'
  }

  if (result.status === 'expired' || result.code === 0 || result.code === 5) {
    return 'expired'
  }

  if (result.status === 'waiting' || result.code === 1) {
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

function platformMembershipStatusLabel(membership?: PlatformMembershipInfo | null) {
  return membership?.statusLabel?.trim() || '未检测到会员信息'
}

function platformMembershipDetailLabel(membership?: PlatformMembershipInfo | null) {
  if (!membership) {
    return '会员摘要仅显示状态，不展示平台凭据。'
  }

  const details = [
    membership.typeLabel?.trim(),
    membership.levelLabel?.trim(),
    membership.expireAt ? `到期 ${formatNeteaseTimestamp(membership.expireAt)}` : '',
  ].filter(Boolean)

  return details.length > 0 ? details.join(' · ') : '会员摘要仅显示状态，不展示平台凭据。'
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

function formatKugouPlaylistMeta(playlist: KugouPlaylistSummary) {
  return [
    `${formatNeteaseCount(playlist.trackCount)} 首`,
    playlist.creatorNickname ? `创建者 ${playlist.creatorNickname}` : '',
    playlist.subscribed ? '收藏歌单' : '',
  ]
    .filter(Boolean)
    .join(' · ')
}

function formatKugouPlaylistUpdate(playlist: KugouPlaylistSummary) {
  const updateTime = Number(playlist.updateTime ?? 0)
  if (!Number.isFinite(updateTime) || updateTime <= 0) {
    return ''
  }

  const date = new Date(updateTime > 10_000_000_000 ? updateTime : updateTime * 1000)
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

function formatNeteaseTrackSubline(track: NeteasePlaylistTrack) {
  return [formatNeteaseTrackArtists(track), formatNeteaseTrackMeta(track)].filter(Boolean).join(' · ')
}

function formatKugouTrackArtists(track: KugouSearchTrack) {
  return track.artists.filter(Boolean).join(' / ') || '未知歌手'
}

function formatKugouTrackDuration(track: KugouSearchTrack) {
  const durationMs = Number(track.durationMs ?? 0)
  if (!Number.isFinite(durationMs) || durationMs <= 0) {
    return '未知时长'
  }

  return formatTime(durationMs / 1000)
}

function formatKugouTrackMeta(track: KugouSearchTrack) {
  return [
    track.album ? `专辑 ${track.album}` : '',
    formatKugouTrackDuration(track),
    track.payType && track.payType > 0 ? '可能受版权限制' : '',
  ]
    .filter(Boolean)
    .join(' · ')
}

function formatKugouTrackSubline(track: KugouSearchTrack) {
  return [formatKugouTrackArtists(track), formatKugouTrackMeta(track)].filter(Boolean).join(' · ')
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

function formatTrackListSubline(track: MusicTrack) {
  return [trackArtistLabel(track), formatTrackListMeta(track)].filter(Boolean).join(' · ')
}

function formatTrackPlaybackQuality(track: MusicTrack) {
  const parts = [
    formatPlaybackLevel(track.playbackLevel),
    formatPlaybackFileType(track.playbackFileType),
    formatPlaybackBitrate(track.playbackBitrate),
    formatPlaybackSize(track.playbackSize),
  ].filter(Boolean)
  return parts.length > 0 ? `音质：${parts.join(' · ')}` : ''
}

function formatPlaybackLevel(level?: string | null) {
  const normalized = level?.trim().toLowerCase()
  if (!normalized) {
    return ''
  }
  const labels: Record<string, string> = {
    jymaster: '超清母带',
    sky: '沉浸环绕',
    jyeffect: '高清环绕',
    hires: 'Hi-Res',
    lossless: '无损',
    exhigh: '极高',
    super: '蝰蛇超清',
    viper_clear: '蝰蛇母带',
    viper_hifi: '蝰蛇HIFI',
    viper_tape: '蝰蛇磁带',
    viper_atmos: '蝰蛇全景声',
    multitrack: '多轨',
    flac: '无损',
    high: '高品',
    '320': '320k',
    '128': '标准',
    standard: '标准',
  }
  return labels[normalized] ?? level?.trim() ?? ''
}

function formatPlaybackFileType(fileType?: string | null) {
  const normalized = fileType?.trim()
  return normalized ? normalized.toUpperCase() : ''
}

function formatPlaybackBitrate(bitrate?: number | null) {
  if (!bitrate || !Number.isFinite(bitrate) || bitrate <= 0) {
    return ''
  }
  const kbps = bitrate >= 1000 ? Math.round(bitrate / 1000) : Math.round(bitrate)
  return `${kbps} kbps`
}

function formatPlaybackSize(size?: number | null) {
  if (!size || !Number.isFinite(size) || size <= 0) {
    return ''
  }
  if (size >= 1024 * 1024) {
    return `${(size / 1024 / 1024).toFixed(1)} MB`
  }
  return `${Math.round(size / 1024)} KB`
}

function formatCurrentTrackDetail(track: MusicTrack) {
  if (track.source === 'netease') {
    return [
      track.album ? `专辑：${track.album}` : '',
      `时长：${formatTrackDuration(track)}`,
      formatTrackPlaybackQuality(track),
      '来源：网易云在线',
    ]
      .filter(Boolean)
      .join(' · ')
  }

  if (track.source === 'kugou') {
    return [
      track.album ? `专辑：${track.album}` : '',
      `时长：${formatTrackDuration(track)}`,
      formatTrackPlaybackQuality(track),
      '来源：酷狗在线',
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

function formatCurrentTrackTooltip(track: MusicTrack) {
  return [
    `歌曲：${track.title || '未命名音乐'}`,
    `歌手：${trackArtistLabel(track)}`,
    track.album ? `专辑：${track.album}` : '',
    `时长：${formatTrackDuration(track)}`,
    formatTrackPlaybackQuality(track),
    `来源：${trackSourceLabel(track)}`,
    `分类：${normalizeMusicCategory(track.category)}`,
    formatTrackTagsLabel(track) ? `标签：${formatTrackTagsLabel(track)}` : '',
    formatTrackMeta(track),
    trackSourceTitle(track),
  ]
    .filter(Boolean)
    .join('\n')
}

function currentTrackTooltipTitle(track: MusicTrack | null) {
  return track
    ? formatCurrentTrackTooltip(track)
    : '未选择音乐\n支持 mp3、wav、flac、m4a、aac、ogg、webm。'
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

function handleCurrentCoverImageError() {
  if (currentTrack.value) {
    currentTrack.value.coverImgUrl = null
  }
}

function immersiveTrackDurationLabel(track: MusicTrack) {
  const trackDuration =
    immersiveTrackActive(track) && durationValue.value > 0
      ? durationValue.value
      : sanitizeTrackDuration(track.duration) ?? 0

  if (trackDuration > 0) {
    return formatTime(trackDuration)
  }

  return track.source === 'netease' || track.source === 'kugou' ? '在线' : '--:--'
}

function immersiveNeteaseTrackDurationLabel(track: NeteasePlaylistTrack) {
  if (neteaseTrackActionId.value === track.id) {
    return '获取中'
  }

  const durationMs = Number(track.durationMs ?? 0)
  return Number.isFinite(durationMs) && durationMs > 0 ? formatTime(durationMs / 1000) : '--:--'
}

function immersiveKugouTrackDurationLabel(track: KugouSearchTrack) {
  if (kugouTrackActionHash.value === track.hash) {
    return '获取中'
  }

  const durationMs = Number(track.durationMs ?? 0)
  return Number.isFinite(durationMs) && durationMs > 0 ? formatTime(durationMs / 1000) : '--:--'
}

function trackSourceLabel(track: MusicTrack) {
  if (track.source === 'netease') {
    return '网易云在线'
  }

  if (track.source === 'kugou') {
    return '酷狗在线'
  }

  return track.sourcePath && track.sourcePath !== track.path ? '存储目录' : '原始位置'
}

function trackSourceTitle(track: MusicTrack) {
  if (track.source === 'netease') {
    return `网易云歌曲 ID：${track.neteaseSongId ?? '未知'}`
  }

  if (track.source === 'kugou') {
    return `酷狗歌曲 Hash：${track.kugouSongHash ?? '未知'}`
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

function resolveImmersiveLyricsLayoutStyle(current: string, previous: string, next: string) {
  const fontScale = lyricFontScaleValue()
  const widthScale = 1
  const lineCapacityScale = clamp(widthScale / fontScale, 0.72, 1.38)
  const currentLines = estimateImmersiveLyricLineCount(
    current,
    Math.max(9, Math.round(14 * lineCapacityScale)),
    IMMERSIVE_LYRIC_MAIN_MAX_LINES,
  )
  const sideLines = Math.max(
    estimateImmersiveLyricLineCount(
      previous,
      Math.max(16, Math.round(24 * lineCapacityScale)),
      IMMERSIVE_LYRIC_SIDE_MAX_LINES,
    ),
    estimateImmersiveLyricLineCount(
      next,
      Math.max(16, Math.round(24 * lineCapacityScale)),
      IMMERSIVE_LYRIC_SIDE_MAX_LINES,
    ),
  )
  const baseMainFontSize =
    currentLines >= 4 ? 38 : currentLines === 3 ? 44 : currentLines === 2 ? 54 : 62
  const mainFontSize = Math.round(baseMainFontSize * fontScale)
  const mainLineHeight = currentLines >= 3 ? 1.16 : 1.1
  const sideFontSize = Math.round((sideLines > 1 ? 20 : 24) * fontScale)
  const mainMinHeight = Math.ceil(currentLines * mainFontSize * mainLineHeight + 58 * fontScale)
  const sideMinHeight = Math.max(34, Math.ceil(sideLines * sideFontSize * 1.34))
  const stackGap = Math.round((currentLines >= 3 ? 26 : currentLines === 2 ? 20 : 16) * (0.9 + fontScale * 0.1))

  return {
    '--immersive-current-line-count': String(currentLines),
    '--immersive-side-line-count': String(sideLines),
    '--immersive-main-font-size': `${mainFontSize}px`,
    '--immersive-main-line-height': String(mainLineHeight),
    '--immersive-main-min-height': `${mainMinHeight}px`,
    '--immersive-side-font-size': `${sideFontSize}px`,
    '--immersive-side-min-height': `${sideMinHeight}px`,
    '--immersive-lyrics-stack-gap': `${stackGap}px`,
  }
}

function resolveImmersiveLyricStageStyle() {
  const defaults = lyricStageDefaultsForPreset('projection')
  const depth = defaults.depth
  const onDjFloat = visualStagePreset.value === 'dj'
  const tilt = clamp(lyricStageTilt.value, 0, 1)
  const glow = clamp(lyricStageGlow.value, 0, 1)
  const verticalOffset = lyricVerticalOffsetPx()
  const distanceOffset = lyricDistanceOffsetPx()
  const distanceScale = lyricDistanceScaleValue()
  const widthScale = 1
  const sideOpacity = lyricSideOpacityValue()
  const progressStrength = 0.55 + defaults.progressGlow * 0.55
  const energy = visualEnergyFrame.value
  const lyric = immersiveLyrics.value
  const envelope = lyricMusicEnvelope.value
  const motionScale = visualReducedMotion.value ? 0 : 1
  const time = visualTimeValue.value
  const activeMusic = playing.value && Boolean(currentTrack.value)
  const lyricActive = lyric.status === 'ready' && !lyric.interlude
  const lyricArc = lyricActive ? Math.sin(Math.PI * clamp(lyric.progress, 0, 1)) : 0
  const musicPulse = activeMusic && !visualReducedMotion.value ? envelope.pulse : 0
  const musicBreath = activeMusic && !visualReducedMotion.value ? envelope.breath : 0
  const musicPhrase = activeMusic && !visualReducedMotion.value ? envelope.phrase : 0
  const musicAir = activeMusic && !visualReducedMotion.value ? envelope.air : 0
  const musicDrift = activeMusic && !visualReducedMotion.value ? envelope.drift : 0
  const musicFocus = activeMusic && !visualReducedMotion.value ? envelope.focus : 0
  const musicDrive = activeMusic
    ? clamp(musicBreath * 0.56 + musicPhrase * 0.18 + musicAir * 0.14 + energy.volume * 0.12, 0, 1)
    : 0
  const bassLift = activeMusic && !visualReducedMotion.value
    ? clamp(musicPulse * 10 + musicPhrase * 5 + musicBreath * 3, 0, onDjFloat ? 16 : 22)
    : 0
  const stageFloatX = activeMusic && !visualReducedMotion.value
    ? Math.sin(time * 0.28 + musicAir * 1.1) * (onDjFloat ? 0.7 + musicAir * 1.4 : 0.9 + musicAir * 1.9) +
      musicDrift * (onDjFloat ? 1.2 : 1.7)
    : 0
  const stageFloatY = activeMusic && !visualReducedMotion.value
    ? Math.cos(time * 0.24 + musicBreath * 1.2) * (onDjFloat ? 0.5 + musicBreath * 1.2 : 0.8 + musicBreath * 1.8)
    : 0
  const musicZLift = activeMusic && !visualReducedMotion.value
    ? onDjFloat
      ? musicDrive * 8 + musicFocus * 7
      : musicDrive * 12 + musicFocus * 8
    : 0
  const roll = activeMusic && !visualReducedMotion.value
    ? (Math.sin(time * 0.16) * 0.26 + musicDrift * 0.18 + (musicAir - musicPulse) * 0.08) * (onDjFloat ? 0.58 : 1)
    : 0
  const audioGlow = clamp(0.1 + glow * 0.34 + musicBreath * 0.24 + musicFocus * 0.18, 0, 1.12)
  const stageAir = activeMusic && !visualReducedMotion.value ? clamp(musicAir * 0.72 + musicPhrase * 0.16, 0, 1) : 0
  const lyricSolar = activeMusic && !visualReducedMotion.value
    ? clamp(
      musicBreath * 0.24 +
      musicAir * 0.24 +
      musicFocus * 0.2 +
      lyricArc * 0.16,
      0,
      1,
    )
    : glow * 0.12
  const lyricReadability = clamp(0.62 + glow * 0.18 + musicFocus * 0.14 + lyricArc * 0.08, 0.58, 0.96)
  const lineLift = activeMusic && !visualReducedMotion.value ? -Math.round(bassLift * (onDjFloat ? 0.4 : 0.5)) : 0
  const presetYOffset = onDjFloat ? DJ_FLOAT_LYRIC_Y_OFFSET : 6
  const presetZOffset = onDjFloat ? DJ_FLOAT_LYRIC_Z_OFFSET : 22
  const stagePresetYOffset = onDjFloat ? 0 : visualStagePreset.value === 'galaxy' ? -8 : 0
  const tiltX = onDjFloat
    ? 5 + tilt * 18 + immersiveStagePitch.value * 0.1
    : (-3.2 + immersiveStagePitch.value * 0.18) * tilt
  const tiltY = onDjFloat
    ? (2.4 - immersiveStageYaw.value * 0.11) * (0.7 + tilt * 0.45)
    : (2.2 - immersiveStageYaw.value * 0.16) * tilt
  const z = 62 + depth * 132 + presetZOffset + distanceOffset + musicZLift
  const scale =
    distanceScale *
    (onDjFloat ? DJ_FLOAT_LYRIC_SCALE : 1) *
    (1 +
      musicDrive * 0.012 * motionScale)

  return {
    '--lyric-stage-x': `${stageFloatX.toFixed(1)}px`,
    '--lyric-stage-y': `${(
      presetYOffset +
      stagePresetYOffset +
      verticalOffset +
      stageFloatY -
      bassLift * (onDjFloat ? 0.12 : 0.18)
    ).toFixed(1)}px`,
    '--lyric-stage-z': `${Math.round(z)}px`,
    '--lyric-stage-scale': scale.toFixed(4),
    '--lyric-stage-tilt-x': `${tiltX.toFixed(2)}deg`,
    '--lyric-stage-tilt-y': `${tiltY.toFixed(2)}deg`,
    '--lyric-stage-roll': `${roll.toFixed(2)}deg`,
    '--lyric-stage-glow-strength': (0.16 + glow * 0.72).toFixed(3),
    '--lyric-stage-audio-glow': audioGlow.toFixed(3),
    '--lyric-stage-air': stageAir.toFixed(3),
    '--lyric-stage-solar': lyricSolar.toFixed(3),
    '--lyric-stage-readability': lyricReadability.toFixed(3),
    '--lyric-stage-line-lift': `${lineLift}px`,
    '--lyric-music-pulse': musicPulse.toFixed(3),
    '--lyric-music-breath': musicBreath.toFixed(3),
    '--lyric-music-phrase': musicPhrase.toFixed(3),
    '--lyric-music-drift': musicDrift.toFixed(3),
    '--lyric-music-focus': musicFocus.toFixed(3),
    '--lyric-stage-shadow-depth': (onDjFloat
      ? 0.62 + musicDrive * 0.2
      : 0.48 + depth * 0.36 + musicDrive * 0.18).toFixed(3),
    '--lyric-stage-floor-width': (onDjFloat
      ? 1.12 + musicDrive * 0.1
      : 0.88 + widthScale * 0.18 + musicDrive * 0.08).toFixed(3),
    '--lyric-appearance-width-px': `${Math.round(940 * widthScale)}px`,
    '--lyric-appearance-width-vw': `${(66 * widthScale).toFixed(1)}vw`,
    '--lyric-side-opacity': sideOpacity.toFixed(3),
    '--lyric-progress-strength': progressStrength.toFixed(3),
  }
}

function estimateImmersiveLyricLineCount(text: string, charsPerLine: number, maxLines: number) {
  const normalized = text.trim()
  if (!normalized) {
    return 1
  }

  const weightedLength = Array.from(normalized).reduce((total, char) => {
    if (/\s/.test(char)) {
      return total + 0.35
    }

    return total + (/[\u0000-\u007f]/.test(char) ? 0.58 : 1)
  }, 0)

  return Math.max(1, Math.min(maxLines, Math.ceil(weightedLength / charsPerLine)))
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

function smoothEnvelopeValue(
  current: number,
  target: number,
  attackSeconds: number,
  releaseSeconds: number,
  deltaSeconds: number,
) {
  const duration = target > current ? attackSeconds : releaseSeconds
  const factor = 1 - Math.exp(-deltaSeconds / Math.max(0.001, duration))
  return current + (target - current) * clamp(factor, 0, 1)
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
      miniEdgeDockClass,
      {
        'music-window-mini': miniPlayerMode,
        'music-window-immersive': immersiveMode,
        'music-window-list-focus': listFocusMode && !miniPlayerMode && !immersiveMode,
      },
    ]"
  >
    <audio
      ref="audio"
      :src="currentTrack?.url"
      preload="auto"
      @loadedmetadata="handleLoadedMetadata"
      @timeupdate="handleTimeUpdate"
      @waiting="handleAudioWaiting"
      @stalled="handleAudioStalled"
      @playing="handleAudioRecovered"
      @ended="handleEnded"
      @error="handleAudioError"
    />

    <section
      v-if="miniPlayerMode"
      ref="miniPlayerElement"
      class="music-mini-player"
      @pointerdown="startDrag"
      @pointerup="scheduleMiniEdgeDockCheck()"
      @pointercancel="scheduleMiniEdgeDockCheck()"
      @pointerenter="revealMiniEdgeDock"
      @pointerleave="scheduleMiniEdgeRehide"
    >
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
        'is-stage-only': immersiveStageOnlyMode,
        'is-stage-dragging': immersiveStageDragging,
        'is-free-camera-active': immersiveFreeCameraActive,
        'is-free-camera-locked': immersiveFreeCameraLocked || immersiveFreeCameraResetting,
        'is-stage-galaxy': visualStagePreset === 'galaxy',
        'is-stage-dj': visualStagePreset === 'dj',
      }"
    >
      <div
        ref="immersiveScene"
        class="music-immersive-scene"
        @pointerdown="startImmersiveStageDrag"
        @pointermove="moveImmersiveStageDrag"
        @pointerup="finishImmersiveStageDrag"
        @pointercancel="finishImmersiveStageDrag"
        @lostpointercapture="finishImmersiveStageDrag"
        @wheel="handleImmersiveSceneWheel"
        @dblclick="resetImmersiveStageView"
      >
        <div class="music-immersive-stage" :style="immersiveStageStyle">
          <div class="music-immersive-stage-backdrop" aria-hidden="true" />
          <MusicVisualizerCanvas
            :frequency-data="visualFrequencyData"
            :energy="visualEnergyFrame"
            :playing="playing"
            :mode="visualMode"
            :spectrum-style="canvasSpectrumStyle"
            :line-style="canvasLineStyle"
            :ripple-style="canvasRippleStyle"
            :intensity="visualIntensity"
            :reduced-motion="visualReducedMotion"
            :theme="resolvedImmersiveTheme"
            :disable-foreground="canvasForegroundDisabled"
          />
          <MusicWebglStarfield
            v-if="webglStarfieldActive"
            :energy="visualEnergyFrame"
            :frequency-data="visualFrequencyData"
            :playing="playing"
            :mode="visualMode"
            :stage-preset="visualStagePreset"
            :spectrum-style="visualSpectrumStyle"
            :line-style="visualLineStyle"
            :ripple-style="visualRippleStyle"
            :intensity="visualIntensity"
            :reduced-motion="visualReducedMotion"
            :stage-tuning="visualStageTuning"
            :theme="resolvedImmersiveTheme"
            :stage-yaw="immersiveStageYaw"
            :stage-pitch="immersiveStagePitch"
            :stage-dragging="immersiveStageDragging"
            :free-camera="immersiveFreeCameraView"
            :lyric-stage="webglLyricStage"
            @webgl-unavailable="handleWebglStarfieldUnavailable"
          />
          <div class="music-immersive-vignette" aria-hidden="true" />

          <section
            v-if="!webglLyricStage.active"
            class="music-immersive-lyrics"
            :class="{
              'is-synced': immersiveLyrics.synced,
              'is-loading': lyricsStatus === 'loading',
              'is-empty': lyricsStatus === 'empty',
              'is-error': lyricsStatus === 'error',
              'is-interlude': immersiveLyrics.interlude,
              'is-reduced-motion': visualReducedMotion,
              'is-idle': !currentTrack,
              'is-stage-only': immersiveStageOnlyMode,
              'is-galaxy-float': visualStagePreset === 'galaxy',
              'is-dj-float': visualStagePreset === 'dj',
            }"
            :style="immersiveLyricsStageStyle"
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
                <p
                  class="music-immersive-lyrics-line current"
                  :style="{ '--lyric-progress': `${Math.round(immersiveLyrics.progress * 1000) / 10}%` }"
                >
                  <span class="music-immersive-lyrics-line-text">
                    {{ immersiveLyrics.current }}
                  </span>
                </p>
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
        </div>
      </div>

      <button
        v-if="immersiveStageOnlyMode"
        type="button"
        class="music-immersive-stage-only-restore music-immersive-icon-button"
        title="显示完整沉浸界面"
        aria-label="显示完整沉浸界面，可按 Esc 退出只看舞台"
        @pointerdown.stop
        @click="setImmersiveStageOnlyMode(false)"
      >
        <svg class="music-action-svg" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <path d="M2.5 12s3.5-6 9.5-6 9.5 6 9.5 6-3.5 6-9.5 6-9.5-6-9.5-6Z" />
          <circle cx="12" cy="12" r="2.7" />
        </svg>
        <span class="music-icon-only-label">显示界面</span>
      </button>

      <header v-if="!immersiveStageOnlyMode" class="music-immersive-header">
        <div>
          <span>{{ visualizerStatusLabel }}</span>
          <h1>{{ currentTrack?.title || '沉浸音乐模式' }}</h1>
          <p>{{ currentTrack ? trackArtistLabel(currentTrack) : '选择音乐后开始可视化' }}</p>
        </div>
        <div class="music-immersive-actions" @pointerdown.stop>
          <button
            type="button"
            class="music-immersive-icon-button"
            :class="{ active: immersiveStageOnlyMode }"
            title="隐藏界面控件，只显示音乐舞台"
            aria-label="只看舞台"
            :aria-pressed="immersiveStageOnlyMode"
            @click="toggleImmersiveStageOnlyMode"
          >
            <svg class="music-action-svg" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
              <path d="M2.5 12s3.5-6 9.5-6 9.5 6 9.5 6-3.5 6-9.5 6-9.5-6-9.5-6Z" />
              <circle cx="12" cy="12" r="2.7" />
            </svg>
            <span class="music-icon-only-label">只看舞台</span>
          </button>
          <button
            type="button"
            class="music-immersive-icon-button"
            title="返回普通播放器"
            aria-label="返回普通播放器"
            @click="setImmersiveMode(false)"
          >
            <svg class="music-action-svg" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
              <path d="M19 12H5" />
              <path d="M12 5l-7 7 7 7" />
            </svg>
            <span class="music-icon-only-label">返回普通播放器</span>
          </button>
          <button type="button" class="window-close" title="隐藏播放器" @click="hideMusicPlayer">
            ×
          </button>
        </div>
      </header>

      <form
        v-if="!immersiveStageOnlyMode"
        ref="immersiveSearchRoot"
        class="music-immersive-search"
        :class="{
          'is-open': immersiveSearchPanelVisible,
          'is-loading': immersiveSearchLoading,
          'has-results': immersiveSearchResults.length > 0,
        }"
        role="search"
        aria-label="沉浸模式音乐搜索"
        @submit.prevent="searchImmersiveSongs"
        @focusin="handleImmersiveSearchFocus"
        @pointerenter="handleImmersiveSearchFocus"
        @pointerdown.stop="handleImmersiveSearchFocus"
      >
        <label class="music-immersive-search-box">
          <svg class="music-immersive-search-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <circle cx="11" cy="11" r="7" />
            <path d="m20 20-4.2-4.2" />
          </svg>
          <input
            ref="immersiveSearchInput"
            v-model="immersiveSearchQuery"
            type="search"
            autocomplete="off"
            spellcheck="false"
            :placeholder="immersiveSearchPlaceholder"
            aria-label="搜索在线音乐"
            @input="handleImmersiveSearchInput"
          />
          <button type="submit" :disabled="immersiveSearchLoading">
            {{ immersiveSearchLoading ? '搜索中' : '搜索' }}
          </button>
        </label>

        <div class="music-immersive-search-platforms" role="group" aria-label="搜索音乐平台">
          <button
            v-for="option in immersiveSearchPlatformOptions"
            :key="option.value"
            type="button"
            :class="{ active: immersiveSearchPlatformSelected(option.value) }"
            :aria-pressed="immersiveSearchPlatformSelected(option.value)"
            @click="toggleImmersiveSearchPlatform(option.value)"
          >
            {{ option.label }}
          </button>
        </div>

        <div
          v-if="immersiveSearchPanelVisible"
          class="music-immersive-search-results"
          :class="{ error: Boolean(immersiveSearchError) }"
        >
          <div class="music-immersive-search-status" role="status">
            {{ immersiveSearchStatusLabel }}
          </div>
          <div
            v-if="immersiveSearchResults.length > 0"
            class="music-immersive-search-list"
            role="list"
            aria-label="沉浸模式搜索结果"
          >
            <button
              v-for="item in immersiveSearchResults"
              :key="item.key"
              type="button"
              class="music-immersive-search-row"
              :class="{
                active: immersiveSearchResultActive(item),
                unavailable: Boolean(immersiveSearchResultUnavailableReason(item)),
              }"
              :disabled="!canPlayImmersiveSearchResult(item)"
              :title="
                immersiveSearchResultUnavailableReason(item)
                  ? playbackFailureDetailTitle(immersiveSearchResultUnavailableReason(item))
                  : `${immersiveSearchResultTitle(item)} - ${immersiveSearchResultArtist(item)}`
              "
              @click="playImmersiveSearchResult(item)"
            >
              <span class="music-immersive-search-source">
                {{ immersiveSearchResultPlatformLabel(item) }}
              </span>
              <span class="music-immersive-search-copy">
                <strong>{{ immersiveSearchResultTitle(item) }}</strong>
                <small>
                  {{
                    immersiveSearchResultUnavailableReason(item)
                      ? compactPlaybackFailureReason(immersiveSearchResultUnavailableReason(item))
                      : immersiveSearchResultArtist(item)
                  }}
                </small>
                <small v-if="immersiveSearchResultAlbum(item)">
                  {{ immersiveSearchResultAlbum(item) }}
                </small>
              </span>
              <span class="music-immersive-search-duration">
                {{
                  immersiveSearchResultUnavailableReason(item)
                    ? '不可播'
                    : immersiveSearchResultDurationLabel(item)
                }}
              </span>
            </button>
          </div>
        </div>
        <button
          type="button"
          class="music-immersive-search-handle"
          aria-label="展开沉浸搜索"
          @click="handleImmersiveSearchFocus"
        />
      </form>

      <div
        v-if="!immersiveStageOnlyMode && (!immersivePlaylistVisible || !immersiveRhythmPanelVisible)"
        class="music-immersive-reopen-actions"
        @pointerdown.stop
      >
        <button
          v-if="!immersivePlaylistVisible"
          type="button"
          class="music-immersive-panel-reopen playlist music-immersive-icon-button"
          title="显示播放列表"
          aria-label="显示播放列表"
          @click="toggleImmersivePlaylistVisible"
        >
          <svg class="music-action-svg" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <path d="M8 6h12" />
            <path d="M8 12h12" />
            <path d="M8 18h12" />
            <path d="M4 6h.01" />
            <path d="M4 12h.01" />
            <path d="M4 18h.01" />
          </svg>
          <span class="music-icon-only-label">显示播放列表</span>
        </button>
        <button
          v-if="!immersiveRhythmPanelVisible"
          type="button"
          class="music-immersive-panel-reopen panel music-immersive-icon-button"
          title="显示舞台与主题面板"
          aria-label="显示舞台与主题面板"
          @click="toggleImmersiveRhythmPanelVisible"
        >
          <svg class="music-action-svg" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <path d="M3 12h3l2-6 4 12 3-8 2 2h4" />
          </svg>
          <span class="music-icon-only-label">显示舞台与主题面板</span>
        </button>
      </div>

      <aside
        v-if="!immersiveStageOnlyMode"
        class="music-immersive-playlist"
        :class="{ 'is-collapsed': !immersivePlaylistVisible }"
        @pointerdown.stop
      >
        <button
          v-if="!immersivePlaylistVisible"
          type="button"
          class="music-immersive-card-icon-button"
          title="显示播放列表"
          aria-label="显示播放列表"
          @click="toggleImmersivePlaylistVisible"
        >
          <svg class="music-action-svg" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <path d="M8 6h12" />
            <path d="M8 12h12" />
            <path d="M8 18h12" />
            <path d="M4 6h.01" />
            <path d="M4 12h.01" />
            <path d="M4 18h.01" />
          </svg>
          <span class="music-icon-only-label">显示播放列表</span>
        </button>

        <template v-else>
        <div class="music-immersive-playlist-heading">
          <div class="music-immersive-card-header">
            <div class="music-immersive-playlist-title">
              <strong>播放列表</strong>
              <span>{{ immersivePlaylistCountLabel }}</span>
            </div>
            <button
              type="button"
              class="music-immersive-card-icon-button"
              title="隐藏播放列表"
              aria-label="隐藏播放列表"
              @click="toggleImmersivePlaylistVisible"
            >
              <svg class="music-action-svg" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
                <path d="M8 6h12" />
                <path d="M8 12h12" />
                <path d="M8 18h12" />
                <path d="M4 6h.01" />
                <path d="M4 12h.01" />
                <path d="M4 18h.01" />
              </svg>
              <span class="music-icon-only-label">隐藏播放列表</span>
            </button>
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
            <button
              type="button"
              :class="{ active: immersivePlaylistSource === 'kugou' }"
              role="tab"
              :aria-selected="immersivePlaylistSource === 'kugou'"
              @click="immersivePlaylistSource = 'kugou'"
            >
              酷狗
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
            :class="{
              active: immersiveNeteaseTrackActive(track),
              unavailable: Boolean(neteaseTrackUnavailableReason(track)),
            }"
            :disabled="!canPlayImmersiveNeteaseTrack(track)"
            :aria-current="immersiveNeteaseTrackActive(track) ? 'true' : undefined"
            :title="
              neteaseTrackUnavailableReason(track)
                ? playbackFailureDetailTitle(neteaseTrackUnavailableReason(track))
                : `${track.name} - ${formatNeteaseTrackArtists(track)}`
            "
            @click="playImmersiveNeteaseTrack(track)"
          >
            <span class="music-immersive-playlist-index">
              {{ immersiveNeteaseTrackActive(track) ? '▶' : index + 1 }}
            </span>
            <span class="music-immersive-playlist-main">
              <strong>{{ track.name }}</strong>
              <small>
                {{
                  neteaseTrackUnavailableReason(track)
                    ? compactPlaybackFailureReason(neteaseTrackUnavailableReason(track))
                    : formatNeteaseTrackArtists(track)
                }}
              </small>
            </span>
            <span class="music-immersive-playlist-duration">
              {{ neteaseTrackUnavailableReason(track) ? '不可播' : immersiveNeteaseTrackDurationLabel(track) }}
            </span>
          </button>
        </div>

        <div
          v-else-if="immersivePlaylistSource === 'kugou' && immersiveKugouPlaylistTracks.length > 0"
          class="music-immersive-playlist-list"
          role="list"
          aria-label="沉浸模式酷狗播放列表"
        >
          <button
            v-for="(track, index) in immersiveKugouPlaylistTracks"
            :key="`kugou-${track.hash}`"
            type="button"
            class="music-immersive-playlist-row online"
            :class="{
              active: immersiveKugouTrackActive(track),
              unavailable: Boolean(kugouTrackUnavailableReason(track)),
            }"
            :disabled="!canPlayImmersiveKugouTrack(track)"
            :aria-current="immersiveKugouTrackActive(track) ? 'true' : undefined"
            :title="
              kugouTrackUnavailableReason(track)
                ? playbackFailureDetailTitle(kugouTrackUnavailableReason(track))
                : `${track.name} - ${formatKugouTrackArtists(track)}`
            "
            @click="playImmersiveKugouTrack(track)"
          >
            <span class="music-immersive-playlist-index">
              {{ immersiveKugouTrackActive(track) ? '▶' : index + 1 }}
            </span>
            <span class="music-immersive-playlist-main">
              <strong>{{ track.name }}</strong>
              <small>
                {{
                  kugouTrackUnavailableReason(track)
                    ? compactPlaybackFailureReason(kugouTrackUnavailableReason(track))
                    : formatKugouTrackArtists(track)
                }}
              </small>
            </span>
            <span class="music-immersive-playlist-duration">
              {{ kugouTrackUnavailableReason(track) ? '不可播' : immersiveKugouTrackDurationLabel(track) }}
            </span>
          </button>
        </div>

        <div v-else class="music-immersive-playlist-empty">
          {{ immersivePlaylistEmptyLabel }}
        </div>
        </template>
      </aside>

      <aside
        v-if="!immersiveStageOnlyMode"
        class="music-immersive-panel"
        :class="{ 'is-collapsed': !immersiveRhythmPanelVisible }"
        @pointerdown.stop
      >
        <button
          v-if="!immersiveRhythmPanelVisible"
          type="button"
          class="music-immersive-card-icon-button"
          title="显示韵律面板"
          aria-label="显示韵律面板"
          @click="toggleImmersiveRhythmPanelVisible"
        >
          <svg class="music-action-svg" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <path d="M3 12h3l2-6 4 12 3-8 2 2h4" />
          </svg>
          <span class="music-icon-only-label">显示韵律面板</span>
        </button>

        <template v-else>
        <div class="music-immersive-panel-heading">
          <div>
            <strong>{{ visualStagePresetLabel }}</strong>
            <span>能量 {{ visualizerEnergyLabel }}</span>
          </div>
          <button
            type="button"
            class="music-immersive-card-icon-button"
            title="隐藏韵律面板"
            aria-label="隐藏韵律面板"
            @click="toggleImmersiveRhythmPanelVisible"
          >
            <svg class="music-action-svg" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
              <path d="M3 12h3l2-6 4 12 3-8 2 2h4" />
            </svg>
            <span class="music-icon-only-label">隐藏韵律面板</span>
          </button>
        </div>

        <div class="music-immersive-panel-section">
          <span>舞台预设</span>
          <div class="music-stage-preset-grid" role="tablist" aria-label="沉浸舞台预设">
            <button
              v-for="option in stagePresetOptions"
              :key="option.value"
              type="button"
              :class="{ active: visualStagePreset === option.value }"
              :title="option.description"
              @click="setVisualStagePreset(option.value)"
            >
              <span class="music-stage-preset-kicker">{{ option.kicker }}</span>
              <span class="music-stage-preset-swatches" aria-hidden="true">
                <span
                  v-for="swatch in option.swatches"
                  :key="swatch"
                  :style="{ background: swatch }"
                />
              </span>
              <span class="music-stage-preset-copy">
                <strong>{{ option.label }}</strong>
                <small>{{ option.description }}</small>
              </span>
              <span class="music-stage-preset-metrics" aria-hidden="true">
                <span v-for="metric in option.metrics" :key="metric">{{ metric }}</span>
              </span>
            </button>
          </div>
        </div>

        <div class="music-immersive-panel-section">
          <span>主题</span>
          <div class="music-immersive-theme-grid" role="tablist" aria-label="沉浸主题风格">
            <button
              v-for="option in immersiveThemeOptions"
              :key="option.value"
              type="button"
              :class="{ active: musicImmersiveThemePreference === option.value }"
              :title="option.description"
              :disabled="immersiveThemeSaving"
              @click="setMusicImmersiveThemePreference(option.value)"
            >
              <span class="music-immersive-theme-card-art" aria-hidden="true">
                <span
                  v-for="swatch in option.swatches"
                  :key="swatch"
                  :style="{ background: swatch }"
                />
              </span>
              <span class="music-immersive-theme-card-copy">
                <strong>{{ option.label }}</strong>
                <small>{{ option.description }}</small>
              </span>
            </button>
          </div>
        </div>

        <div class="music-immersive-panel-section music-stage-parameter-card">
          <div class="music-stage-parameter-card-heading">
            <span>参数调节</span>
            <small>{{ visualStagePresetLabel }}</small>
          </div>
          <label class="music-immersive-range music-stage-intensity-range">
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
          <div class="music-stage-tuning-grid">
            <label
              v-for="option in stageTuningOptions"
              :key="option.key"
              class="music-immersive-range music-stage-tuning-range"
            >
              <span>{{ option.label }}</span>
              <input
                v-model.number="visualStageTuning[option.key]"
                type="range"
                :min="option.min"
                :max="option.max"
                :step="option.step"
                :aria-label="option.ariaLabel"
              />
              <output>{{ formatStageTuningValue(visualStageTuning[option.key]) }}</output>
            </label>
          </div>
          <div class="music-stage-tuning-actions">
            <button type="button" @click="resetVisualStageTuning">重置参数</button>
          </div>
        </div>

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

        <div class="music-immersive-panel-section music-lyric-stage-panel">
          <span>歌词舞台</span>
          <div class="music-lyric-stage-controls">
            <label class="music-immersive-range music-lyric-stage-range">
              <span>倾角</span>
              <input
                v-model.number="lyricStageTilt"
                type="range"
                min="0"
                max="1"
                step="0.01"
                aria-label="歌词舞台倾角"
              />
              <output>{{ lyricStageTiltLabel }}</output>
            </label>
            <label class="music-immersive-range music-lyric-stage-range">
              <span>溢光</span>
              <input
                v-model.number="lyricStageGlow"
                type="range"
                min="0"
                max="1"
                step="0.01"
                aria-label="歌词舞台溢光"
              />
              <output>{{ lyricStageGlowLabel }}</output>
            </label>
          </div>
          <div class="music-lyric-stage-controls music-lyric-appearance-controls">
            <label class="music-immersive-range music-lyric-stage-range">
              <span>字号</span>
              <input
                v-model.number="lyricStageFontScale"
                type="range"
                min="0"
                max="1"
                step="0.01"
                aria-label="沉浸歌词字号"
              />
              <output>{{ lyricStageFontScaleLabel }}</output>
            </label>
            <label class="music-immersive-range music-lyric-stage-range">
              <span>位置</span>
              <input
                v-model.number="lyricStageVertical"
                type="range"
                min="0"
                max="1"
                step="0.01"
                aria-label="沉浸歌词垂直位置"
              />
              <output>{{ lyricStageVerticalLabel }}</output>
            </label>
            <label class="music-immersive-range music-lyric-stage-range">
              <span>远近</span>
              <input
                v-model.number="lyricStageDistance"
                type="range"
                min="0"
                max="1"
                step="0.01"
                aria-label="沉浸歌词远近"
              />
              <output>{{ lyricStageDistanceLabel }}</output>
            </label>
            <label class="music-immersive-range music-lyric-stage-range">
              <span>弱化</span>
              <input
                v-model.number="lyricStageSideOpacity"
                type="range"
                min="0"
                max="1"
                step="0.01"
                aria-label="沉浸歌词上下句弱化"
              />
              <output>{{ lyricStageSideOpacityLabel }}</output>
            </label>
          </div>
          <div class="music-stage-tuning-actions music-lyric-stage-actions">
            <button type="button" @click="resetLyricStageParameters">重置歌词</button>
          </div>
        </div>

        <label class="music-immersive-check">
          <input v-model="visualReducedMotion" type="checkbox" />
          <span>降低动态</span>
        </label>

        <p
          v-if="immersiveThemeError || beatMapError || (analyzerError && !beatMapMatchesCurrentTrack)"
          class="music-immersive-warning"
        >
          {{ immersiveThemeError || visualizerHintLabel }}
        </p>
        <p v-else class="music-immersive-hint">
          {{ visualizerHintLabel }}
        </p>
        </template>
      </aside>

      <div v-if="!currentTrack && !immersiveStageOnlyMode" class="music-immersive-empty" @pointerdown.stop>
        <strong>还没有正在播放的音乐</strong>
        <span>返回播放器添加或选择本地歌曲。</span>
        <button type="button" @click="setImmersiveMode(false)">返回播放器</button>
      </div>

      <footer v-if="!immersiveStageOnlyMode" class="music-immersive-controls" @pointerdown.stop>
        <div class="music-immersive-main-controls">
          <button
            type="button"
            class="music-control-icon"
            :disabled="!hasTracks"
            title="上一首"
            aria-label="上一首"
            @click="playPrevious"
          >
            <span aria-hidden="true">‹</span>
          </button>
          <button
            type="button"
            class="music-control-icon music-play-button"
            :disabled="!hasTracks"
            :title="playing ? '暂停' : '播放'"
            :aria-label="playing ? '暂停' : '播放'"
            @click="togglePlay"
          >
            <span aria-hidden="true">{{ playing ? 'Ⅱ' : '▶' }}</span>
          </button>
          <button
            type="button"
            class="music-control-icon"
            :disabled="!hasTracks"
            title="下一首"
            aria-label="下一首"
            @click="playNext(true)"
          >
            <span aria-hidden="true">›</span>
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

        <div
          class="music-immersive-quality-control"
          :class="{ open: immersiveQualityMenuOpen }"
          @focusout="closeImmersiveQualityMenuOnFocusOut"
        >
          <button
            type="button"
            class="music-immersive-quality-trigger"
            :class="{ active: immersiveQualityMenuOpen }"
            :aria-expanded="immersiveQualityMenuOpen"
            aria-haspopup="listbox"
            aria-label="切换沉浸模式在线播放音质"
            :title="`${onlinePlaybackQualityPlatformLabel}：${onlinePlaybackQualityLabel}`"
            :disabled="onlinePlaybackQualitySwitching"
            @click="toggleImmersiveQualityMenu"
          >
            <span>{{ onlinePlaybackQualityPlatformLabel }}</span>
            <strong>{{ onlinePlaybackQualityLabel }}</strong>
          </button>
          <div
            v-if="immersiveQualityMenuOpen"
            class="music-immersive-quality-options"
            role="listbox"
            aria-label="在线播放音质列表"
          >
            <button
              v-for="option in onlinePlaybackQualityOptions"
              :key="`immersive-quality-${option.value}`"
              type="button"
              :class="{
                active: activeOnlinePlaybackQuality === option.value,
                unavailable: onlinePlaybackQualityOptionDisabled(option),
                unknown: option.availabilityStatus === 'unknown',
              }"
              role="option"
              :aria-selected="activeOnlinePlaybackQuality === option.value"
              :aria-label="`切换在线播放音质为${option.label}`"
              :title="onlinePlaybackQualityOptionTitle(option)"
              :disabled="onlinePlaybackQualitySwitching || onlinePlaybackQualityOptionDisabled(option)"
              @click="selectImmersiveOnlinePlaybackQuality(option.value)"
            >
              <span>{{ option.label }}</span>
              <small>{{ option.description }}</small>
            </button>
          </div>
        </div>

        <div class="music-immersive-options">
          <button
            type="button"
            class="music-control-icon"
            :class="{ active: shuffleEnabled }"
            :disabled="playbackListTrackCount < 2"
            title="随机播放"
            aria-label="随机播放"
            @click="shuffleEnabled = !shuffleEnabled"
          >
            <span aria-hidden="true">⇄</span>
          </button>
          <button
            type="button"
            class="music-control-icon"
            :disabled="!hasTracks"
            :title="repeatModeLabel"
            :aria-label="repeatModeLabel"
            @click="toggleRepeatMode"
          >
            <span aria-hidden="true">{{ repeatModeIcon }}</span>
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
        <p>{{ currentTrack ? `正在播放${currentTrackPlatformLabel}` : '选择音频开始播放' }}</p>
      </div>
      <div class="music-header-actions" @pointerdown="stopHeaderDrag">
        <button
          type="button"
          :class="{ active: listFocusMode }"
          :title="listFocusMode ? '恢复播放工作台布局' : '扩大歌曲列表浏览区'"
          :aria-pressed="listFocusMode"
          @click="listFocusMode = !listFocusMode"
        >
          列表
        </button>
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

    <section class="music-standard-shell">
      <section v-if="listFocusMode" class="music-list-player-bar" aria-label="列表模式播放控制">
        <div class="music-list-now">
          <div class="music-cover-frame music-list-cover-frame" aria-hidden="true">
            <img
              v-if="currentTrack?.coverImgUrl"
              class="music-cover-image"
              :src="currentTrack.coverImgUrl"
              alt=""
              draggable="false"
              referrerpolicy="no-referrer"
              @error="handleCurrentCoverImageError"
            />
            <div v-else class="music-disc" :class="{ 'music-disc-playing': playing }">
              <span />
            </div>
          </div>
          <div class="music-list-now-copy">
            <strong>{{ currentTrack?.title || '未选择音乐' }}</strong>
            <small>
              {{ currentTrack ? `${trackArtistLabel(currentTrack)} · ${currentTrackPlatformLabel}` : '选择歌曲后开始播放' }}
            </small>
          </div>
        </div>
        <div class="music-main-controls">
          <button
            type="button"
            class="music-control-icon"
            :disabled="!hasTracks"
            title="上一首"
            aria-label="上一首"
            @click="playPrevious"
          >
            <span aria-hidden="true">‹</span>
          </button>
          <button
            type="button"
            class="music-control-icon music-play-button"
            :disabled="!hasTracks"
            :title="playing ? '暂停' : '播放'"
            :aria-label="playing ? '暂停' : '播放'"
            @click="togglePlay"
          >
            <span aria-hidden="true">{{ playing ? 'Ⅱ' : '▶' }}</span>
          </button>
          <button
            type="button"
            class="music-control-icon"
            :disabled="!hasTracks"
            title="下一首"
            aria-label="下一首"
            @click="playNext(true)"
          >
            <span aria-hidden="true">›</span>
          </button>
        </div>
        <label class="music-volume-row">
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
        <div class="music-option-row">
          <button
            type="button"
            class="music-control-icon"
            :class="{ active: shuffleEnabled }"
            :disabled="playbackListTrackCount < 2"
            title="随机播放"
            aria-label="随机播放"
            @click="shuffleEnabled = !shuffleEnabled"
          >
            <span aria-hidden="true">⇄</span>
          </button>
          <button
            type="button"
            class="music-control-icon"
            :disabled="!hasTracks"
            :title="repeatModeLabel"
            :aria-label="repeatModeLabel"
            @click="toggleRepeatMode"
          >
            <span aria-hidden="true">{{ repeatModeIcon }}</span>
          </button>
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
        </div>
        <div v-if="playerStatus || playerErrorDisplay" class="music-list-feedback">
          <span v-if="playerStatus">{{ playerStatus }}</span>
          <div
            v-if="playerErrorDisplay"
            class="music-error-card music-error-card-compact"
            role="alert"
          >
            <div class="music-error-card-main">
              <strong>{{ playerErrorDisplay.title }}</strong>
              <span>{{ playerErrorDisplay.summary }}</span>
            </div>
            <details v-if="playerErrorDisplay.detail" class="music-error-detail">
              <summary>诊断详情</summary>
              <pre>{{ playerErrorDisplay.detail }}</pre>
            </details>
          </div>
        </div>
      </section>
      <aside class="music-player-panel" aria-label="当前播放和控制">
        <section class="music-now">
          <div class="music-cover-frame" aria-hidden="true">
            <img
              v-if="currentTrack?.coverImgUrl"
              class="music-cover-image"
              :src="currentTrack.coverImgUrl"
              alt=""
              draggable="false"
              referrerpolicy="no-referrer"
              @error="handleCurrentCoverImageError"
            />
            <div v-else class="music-disc" :class="{ 'music-disc-playing': playing }">
              <span />
            </div>
          </div>
          <div
            class="music-current-copy"
            :title="currentTrackTooltipTitle(currentTrack)"
            :aria-label="currentTrackTooltipTitle(currentTrack)"
          >
            <strong :title="currentTrackTooltipTitle(currentTrack)">{{ currentTrack?.title || '未选择音乐' }}</strong>
            <small v-if="currentTrack" class="music-current-artist" :title="currentTrackTooltipTitle(currentTrack)">
              {{ trackArtistLabel(currentTrack) }}
            </small>
            <small v-if="currentTrack" class="music-current-meta" :title="currentTrackTooltipTitle(currentTrack)">
              {{ formatCurrentTrackDetail(currentTrack) }}
            </small>
            <small v-else class="music-current-meta" :title="currentTrackTooltipTitle(null)">
              支持 mp3、wav、flac、m4a、aac、ogg、webm。
            </small>
          </div>
        </section>

        <section class="music-controls" aria-label="播放控制">
          <div class="music-control-strip">
            <div class="music-main-controls">
              <button
                type="button"
                class="music-control-icon"
                :disabled="!hasTracks"
                title="上一首"
                aria-label="上一首"
                @click="playPrevious"
              >
                <span aria-hidden="true">‹</span>
              </button>
              <button
                type="button"
                class="music-control-icon music-play-button"
                :disabled="!hasTracks"
                :title="playing ? '暂停' : '播放'"
                :aria-label="playing ? '暂停' : '播放'"
                @click="togglePlay"
              >
                <span aria-hidden="true">{{ playing ? 'Ⅱ' : '▶' }}</span>
              </button>
              <button
                type="button"
                class="music-control-icon"
                :disabled="!hasTracks"
                title="下一首"
                aria-label="下一首"
                @click="playNext(true)"
              >
                <span aria-hidden="true">›</span>
              </button>
            </div>

            <label class="music-volume-row">
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

          <div class="music-option-row">
            <button
              type="button"
              class="music-control-icon"
              :class="{ active: shuffleEnabled }"
              :disabled="playbackListTrackCount < 2"
              title="随机播放"
              aria-label="随机播放"
              @click="shuffleEnabled = !shuffleEnabled"
            >
              <span aria-hidden="true">⇄</span>
            </button>
            <button
              type="button"
              class="music-control-icon"
              :disabled="!hasTracks"
              :title="repeatModeLabel"
              :aria-label="repeatModeLabel"
              @click="toggleRepeatMode"
            >
              <span aria-hidden="true">{{ repeatModeIcon }}</span>
            </button>
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
          </div>
        </section>

        <div v-if="playerStatus || playerErrorDisplay" class="music-feedback-stack">
          <p v-if="playerStatus" class="music-status">{{ playerStatus }}</p>
          <div v-if="playerErrorDisplay" class="music-error-card" role="alert">
            <div class="music-error-card-main">
              <strong>{{ playerErrorDisplay.title }}</strong>
              <span>{{ playerErrorDisplay.summary }}</span>
            </div>
            <div v-if="playerErrorDisplay.hints.length" class="music-error-actions">
              <span v-for="hint in playerErrorDisplay.hints" :key="hint">{{ hint }}</span>
            </div>
            <details
              v-if="playerErrorDisplay.detail"
              class="music-error-detail"
              :open="playerErrorDetailOpen"
              @toggle="handlePlayerErrorDetailToggle"
            >
              <summary>诊断详情</summary>
              <pre>{{ playerErrorDisplay.detail }}</pre>
            </details>
          </div>
        </div>

        <section class="music-playlist-section music-left-playlists" aria-label="我的歌单">
          <header class="music-playlist-section-header">
            <div>
              <strong>我的歌单</strong>
              <span v-if="activePanelView === 'library'">{{ leftLocalPlaylistStatusLabel }}</span>
              <span v-else-if="activePanelView === 'netease'">{{ leftNeteasePlaylistStatusLabel }}</span>
              <span v-else>{{ leftKugouPlaylistStatusLabel }}</span>
            </div>
          </header>

          <div class="music-left-playlist-groups">
            <section
              v-if="activePanelView === 'library'"
              class="music-left-playlist-group"
              aria-label="本地我的歌单"
            >
              <header class="music-left-playlist-group-header">
                <div>
                  <strong>本地我的歌单</strong>
                  <span>{{ leftLocalPlaylistStatusLabel }}</span>
                </div>
              </header>

              <form class="music-playlist-create" @submit.prevent="createCustomPlaylist">
                <input
                  v-model="newPlaylistName"
                  autocomplete="off"
                  placeholder="新建本地歌单"
                  aria-label="新建本地歌单"
                />
                <button type="submit">新建</button>
              </form>

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
                    <span aria-hidden="true">本</span>
                    <div>
                      <strong>{{ playlist.name }}</strong>
                      <small>{{ playlistTrackCountLabel(playlist) }}</small>
                      <small>{{ playlistPreviewLabel(playlist) }}</small>
                    </div>
                    <span class="music-playlist-card-state" aria-hidden="true">
                      {{ activeCustomPlaylistId === playlist.id ? '展开' : '›' }}
                    </span>
                  </button>

                  <div class="music-custom-playlist-actions">
                    <button type="button" @click="queueCustomPlaylist(playlist)">队列</button>
                    <button type="button" @click="openPlaylistTrackPicker(playlist)">加歌</button>
                    <button type="button" @click="renameCustomPlaylistWithPrompt(playlist)">改名</button>
                    <button type="button" @click="deleteCustomPlaylist(playlist.id)">删除</button>
                  </div>

                  <div
                    v-if="activeCustomPlaylistId === playlist.id"
                    class="music-custom-playlist-detail"
                  >
                    <div
                      v-for="track in customPlaylistTracks(playlist)"
                      :key="track.id"
                      class="music-custom-playlist-track"
                      draggable="true"
                      @dragstart="startPlaylistTrackDrag(track.id, $event)"
                      @dragover.prevent
                      @drop="dropPlaylistTrack(playlist, track.id, $event)"
                      @dragend="finishPlaylistTrackDrag"
                    >
                      <button type="button" @click="playCustomPlaylistTrack(playlist, track.id)">
                        <span>{{ track.title }}</span>
                        <small>{{ trackArtistLabel(track) }}</small>
                      </button>
                      <button
                        type="button"
                        title="从歌单移除"
                        aria-label="从歌单移除"
                        @click="removeTrackFromPlaylist(playlist, track.id)"
                      >
                        ×
                      </button>
                    </div>

                    <p v-if="customPlaylistTracks(playlist).length === 0" class="music-playlist-empty">
                      这个歌单还没有歌曲。
                    </p>
                  </div>
                </article>
              </div>

              <div v-else class="music-empty music-playlist-empty">
                <strong>还没有本地歌单</strong>
                <span>输入名称后新建歌单。</span>
              </div>
            </section>

            <section
              v-else-if="activePanelView === 'netease'"
              class="music-left-playlist-group"
              aria-label="网易云我的歌单"
            >
              <header class="music-left-playlist-group-header">
                <div>
                  <strong>网易云我的歌单</strong>
                  <span>{{ leftNeteasePlaylistStatusLabel }}</span>
                </div>
                <button
                  v-if="neteaseLoggedIn"
                  type="button"
                  :disabled="neteasePlaylistsLoading || neteasePlaylistDetailLoading"
                  @click="refreshNeteasePlaylistsFromLeft"
                >
                  {{ neteasePlaylistsLoading ? '读取中' : '刷新' }}
                </button>
                <button
                  v-else
                  type="button"
                  :disabled="neteaseLoginBusy"
                  @click="openNeteaseLoginFromLeft"
                >
                  登录
                </button>
              </header>

              <div
                v-if="neteaseLoggedIn && neteasePlaylists.length > 0"
                class="music-left-source-playlists"
              >
                <article
                  v-for="playlist in neteasePlaylists"
                  :key="playlist.id"
                  class="music-left-source-playlist-card"
                  :class="{ active: playlist.id === neteaseSelectedPlaylistId }"
                >
                  <button
                    type="button"
                    class="music-left-source-playlist-summary"
                    :disabled="neteasePlaylistDetailLoading"
                    @click="openNeteasePlaylistFromLeft(playlist)"
                  >
                    <img
                      v-if="playlist.coverImgUrl"
                      :src="playlist.coverImgUrl"
                      alt=""
                      referrerpolicy="no-referrer"
                    />
                    <span v-else aria-hidden="true">云</span>
                    <div>
                      <strong>{{ playlist.name }}</strong>
                      <small>{{ formatNeteasePlaylistMeta(playlist) }}</small>
                      <small v-if="formatNeteasePlaylistUpdate(playlist)">
                        {{ formatNeteasePlaylistUpdate(playlist) }}
                      </small>
                    </div>
                    <span class="music-playlist-card-state" aria-hidden="true">
                      {{ playlist.id === neteaseSelectedPlaylistId ? '当前' : '›' }}
                    </span>
                  </button>
                </article>
              </div>

              <div v-else class="music-left-playlist-empty">
                <strong>{{ neteaseLoggedIn ? '暂无网易云歌单' : '未登录网易云' }}</strong>
                <span>{{ neteaseLoggedIn ? '点击刷新读取我的歌单。' : '登录后显示网易云我的歌单。' }}</span>
              </div>
            </section>

            <section
              v-else-if="activePanelView === 'kugou'"
              class="music-left-playlist-group"
              aria-label="酷狗我的歌单"
            >
              <header class="music-left-playlist-group-header">
                <div>
                  <strong>酷狗我的歌单</strong>
                  <span>{{ leftKugouPlaylistStatusLabel }}</span>
                </div>
                <button
                  v-if="kugouLoggedIn"
                  type="button"
                  :disabled="kugouPlaylistsLoading || kugouPlaylistDetailLoading"
                  @click="refreshKugouPlaylistsFromLeft"
                >
                  {{ kugouPlaylistsLoading ? '读取中' : '刷新' }}
                </button>
                <button
                  v-else
                  type="button"
                  :disabled="kugouLoginBusy"
                  @click="openKugouLoginFromLeft"
                >
                  登录
                </button>
              </header>

              <div
                v-if="kugouLoggedIn && kugouPlaylists.length > 0"
                class="music-left-source-playlists"
              >
                <article
                  v-for="playlist in kugouPlaylists"
                  :key="playlist.listId"
                  class="music-left-source-playlist-card"
                  :class="{ active: playlist.listId === kugouSelectedPlaylistId }"
                >
                  <button
                    type="button"
                    class="music-left-source-playlist-summary"
                    :disabled="kugouPlaylistDetailLoading"
                    @click="openKugouPlaylistFromLeft(playlist)"
                  >
                    <img
                      v-if="playlist.coverImgUrl"
                      :src="playlist.coverImgUrl"
                      alt=""
                      referrerpolicy="no-referrer"
                    />
                    <span v-else aria-hidden="true">酷</span>
                    <div>
                      <strong>{{ playlist.name }}</strong>
                      <small>{{ formatKugouPlaylistMeta(playlist) }}</small>
                      <small v-if="formatKugouPlaylistUpdate(playlist)">
                        {{ formatKugouPlaylistUpdate(playlist) }}
                      </small>
                    </div>
                    <span class="music-playlist-card-state" aria-hidden="true">
                      {{ playlist.listId === kugouSelectedPlaylistId ? '当前' : '›' }}
                    </span>
                  </button>
                </article>
              </div>

              <div v-else class="music-left-playlist-empty">
                <strong>{{ kugouLoggedIn ? '暂无酷狗歌单' : '未登录酷狗' }}</strong>
                <span>{{ kugouLoggedIn ? '点击刷新读取我的歌单。' : '登录后显示酷狗我的歌单。' }}</span>
              </div>

              <div class="music-left-playlist-subgroup">
                <header class="music-left-playlist-group-header">
                  <div>
                    <strong>酷狗推荐歌单</strong>
                    <span>{{ leftKugouRecommendedPlaylistStatusLabel }}</span>
                  </div>
                  <button
                    type="button"
                    :disabled="kugouRecommendedPlaylistsLoading || kugouPlaylistDetailLoading"
                    @click="refreshKugouRecommendedPlaylistsFromLeft(false)"
                  >
                    {{ kugouRecommendedPlaylistsLoading ? '读取中' : '刷新' }}
                  </button>
                </header>

                <p
                  v-if="kugouRecommendedPlaylistError || kugouRecommendedPlaylistNotice"
                  class="music-platform-message music-left-playlist-message"
                  :class="{ error: Boolean(kugouRecommendedPlaylistError) }"
                >
                  {{ kugouRecommendedPlaylistError || kugouRecommendedPlaylistNotice }}
                </p>

                <div
                  v-if="kugouRecommendedPlaylists.length > 0"
                  class="music-left-source-playlists"
                >
                  <article
                    v-for="playlist in kugouRecommendedPlaylists"
                    :key="`kugou-recommend-${kugouPlaylistKey(playlist)}`"
                    class="music-left-source-playlist-card"
                    :class="{
                      active:
                        kugouSelectedContentSource === 'recommended' &&
                        kugouPlaylistKey(playlist) === kugouSelectedRecommendedPlaylistId,
                    }"
                  >
                    <button
                      type="button"
                      class="music-left-source-playlist-summary"
                      :disabled="kugouPlaylistDetailLoading"
                      @click="openKugouRecommendedPlaylistFromLeft(playlist)"
                    >
                      <img
                        v-if="playlist.coverImgUrl"
                        :src="playlist.coverImgUrl"
                        alt=""
                        referrerpolicy="no-referrer"
                      />
                      <span v-else aria-hidden="true">荐</span>
                      <div>
                        <strong>{{ playlist.name }}</strong>
                        <small>{{ formatKugouPlaylistMeta(playlist) }}</small>
                        <small v-if="formatKugouPlaylistUpdate(playlist)">
                          {{ formatKugouPlaylistUpdate(playlist) }}
                        </small>
                      </div>
                      <span class="music-playlist-card-state" aria-hidden="true">
                        {{
                          kugouSelectedContentSource === 'recommended' &&
                          kugouPlaylistKey(playlist) === kugouSelectedRecommendedPlaylistId
                            ? '当前'
                            : '›'
                        }}
                      </span>
                    </button>
                  </article>
                </div>

                <div v-else class="music-left-playlist-empty">
                  <strong>暂无推荐歌单</strong>
                  <span>点击刷新读取酷狗公开推荐歌单。</span>
                </div>

                <button
                  v-if="kugouRecommendedPlaylistHasMore"
                  type="button"
                  class="music-platform-load-more music-left-playlist-load-more"
                  :disabled="kugouRecommendedPlaylistsLoading"
                  @click="refreshKugouRecommendedPlaylistsFromLeft(true)"
                >
                  {{ kugouRecommendedPlaylistsLoading ? '加载中' : '加载更多推荐' }}
                </button>
              </div>
            </section>
          </div>
        </section>
      </aside>

      <section class="music-content-panel" aria-label="音乐内容">
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

      <div v-if="activePanelView === 'netease'" class="music-platform-panel">
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
            <div class="music-platform-profile-main">
              <strong>{{ neteaseProfile.nickname }}</strong>
              <span>用户 ID {{ neteaseProfile.userId }}</span>
              <small v-if="neteaseLoginStatus?.savedAt">
                本机保存于 {{ formatNeteaseTimestamp(neteaseLoginStatus.savedAt) }}
              </small>
            </div>
            <span
              class="music-platform-membership"
              :class="{ active: neteaseProfile.membership?.active }"
            >
              <span>{{ platformMembershipStatusLabel(neteaseProfile.membership) }}</span>
              <small>{{ platformMembershipDetailLabel(neteaseProfile.membership) }}</small>
            </span>
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

        <section class="music-platform-card">
          <header class="music-platform-section-header">
            <div>
              <strong>搜索网易云</strong>
              <span>{{ neteaseSearchStatusDetail }}</span>
            </div>
            <span>{{ neteaseSearchLoadedLabel }}</span>
          </header>

          <div class="music-platform-search-row">
            <label class="music-platform-search">
              <span>关键词</span>
              <input
                v-model="neteaseSearchQuery"
                autocomplete="off"
                placeholder="搜索歌曲、歌手或专辑"
                @keydown.enter="searchNeteaseSongs()"
              />
            </label>
            <button type="button" :disabled="neteaseSearchLoading" @click="searchNeteaseSongs()">
              {{ neteaseSearchLoading ? '搜索中' : '搜索' }}
            </button>
          </div>

          <p
            v-if="neteaseSearchError || neteaseSearchNotice"
            class="music-platform-message"
            :class="{ error: Boolean(neteaseSearchError) }"
          >
            {{ neteaseSearchStatusDetail }}
          </p>

          <div v-if="neteaseSearchLoading && neteaseSearchTracks.length === 0" class="music-platform-empty">
            <strong>正在搜索网易云音乐</strong>
            <span>搜索结果只保存在当前运行状态，不写入本机曲库。</span>
          </div>

          <div v-else-if="neteaseSearchResult && neteaseSearchTracks.length === 0" class="music-platform-empty">
            <strong>暂无网易云结果</strong>
            <span>换一个关键词再试。</span>
          </div>

          <div v-else-if="neteaseSearchTracks.length > 0" class="music-platform-track-list">
            <article
              v-for="(track, index) in neteaseSearchTracks"
              :key="`netease-search-${track.id}-${index}`"
              class="music-platform-track-row"
              :class="{
                active: currentTrack?.source === 'netease' && currentTrack.neteaseSongId === track.id,
                unavailable: Boolean(neteaseTrackUnavailableReason(track)),
              }"
            >
              <span class="music-platform-track-index">{{ index + 1 }}</span>
              <img
                v-if="track.coverImgUrl"
                :src="track.coverImgUrl"
                alt=""
                referrerpolicy="no-referrer"
              />
              <span v-else class="music-platform-track-cover" aria-hidden="true">云</span>
              <span class="music-platform-track-copy">
                <strong>{{ track.name }}</strong>
                <small :title="formatNeteaseTrackSubline(track)">
                  {{ formatNeteaseTrackSubline(track) }}
                </small>
                <small
                  v-if="neteaseTrackUnavailableReason(track)"
                  class="music-platform-track-warning"
                  :title="playbackFailureDetailTitle(neteaseTrackUnavailableReason(track))"
                >
                  不可播放：{{ compactPlaybackFailureReason(neteaseTrackUnavailableReason(track)) }}
                </small>
              </span>
              <span class="music-platform-track-actions">
                <button
                  type="button"
                  :disabled="!neteaseLoggedIn || (neteaseLyricsLoading && neteaseLyricsTrack?.id === track.id)"
                  @click="showNeteaseLyrics(track)"
                >
                  {{ neteaseLyricsLoading && neteaseLyricsTrack?.id === track.id ? '读取中' : '歌词' }}
                </button>
                <button
                  type="button"
                  :disabled="!neteaseLoggedIn || neteaseTrackActionId === track.id"
                  @click="playNeteaseTrack(track, neteaseSearchTracks)"
                >
                  {{ neteaseTrackActionId === track.id ? '获取中' : neteaseTrackUnavailableReason(track) ? '重试' : '播放' }}
                </button>
              </span>
            </article>
          </div>

          <button
            v-if="neteaseSearchHasMore"
            type="button"
            class="music-platform-load-more"
            :disabled="neteaseSearchLoading"
            @click="searchNeteaseSongs(true)"
          >
            {{ neteaseSearchLoading ? '加载中' : `加载更多（${neteaseSearchLoadedLabel}）` }}
          </button>
        </section>

        <section v-if="neteaseLoggedIn" class="music-platform-card">
          <header class="music-platform-section-header">
            <div>
              <strong>{{ neteaseSelectedPlaylist?.name || '当前歌单' }}</strong>
              <span>{{ neteasePlaylistStatusLabel }}</span>
            </div>
            <span v-if="neteaseSelectedPlaylist">
              {{ neteasePlaylistLoadedLabel }}
            </span>
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
            <span>在左下角点击“刷新”读取网易云我的歌单。</span>
          </div>

          <section
            v-else-if="neteaseSelectedPlaylist"
            ref="neteasePlaylistDetailSection"
            class="music-platform-detail"
          >
            <header class="music-platform-section-header">
              <div>
                <strong>{{ neteaseSelectedPlaylist.name }}</strong>
                <span>{{ neteasePlaylistLoadedLabel }}</span>
              </div>
              <span v-if="neteasePlaylistHasMore">可继续加载</span>
            </header>

            <div
              v-if="neteasePlaylistDetailLoading && !neteasePlaylistDetail?.tracks.length"
              class="music-platform-empty"
            >
              <strong>正在读取歌曲摘要</strong>
              <span>读取完成后会显示歌曲、歌手、专辑和时长。</span>
            </div>

            <div
              v-if="neteasePlaylistDetail?.tracks.length"
              class="music-platform-track-list"
            >
              <article
                v-for="(track, index) in neteasePlaylistDetail.tracks"
                :key="`${track.id}-${index}`"
                class="music-platform-track-row"
                :class="{
                  active: currentTrack?.source === 'netease' && currentTrack.neteaseSongId === track.id,
                  unavailable: Boolean(neteaseTrackUnavailableReason(track)),
                }"
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
                  <small :title="formatNeteaseTrackSubline(track)">
                    {{ formatNeteaseTrackSubline(track) }}
                  </small>
                  <small
                    v-if="neteaseTrackUnavailableReason(track)"
                    class="music-platform-track-warning"
                    :title="playbackFailureDetailTitle(neteaseTrackUnavailableReason(track))"
                  >
                    不可播放：{{ compactPlaybackFailureReason(neteaseTrackUnavailableReason(track)) }}
                  </small>
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
                    @click="playNeteaseTrack(track, neteasePlaylistDetail?.tracks ?? [])"
                  >
                    {{ neteaseTrackActionId === track.id ? '获取中' : neteaseTrackUnavailableReason(track) ? '重试' : '播放' }}
                  </button>
                </span>
              </article>
            </div>

            <button
              v-if="neteasePlaylistHasMore"
              type="button"
              class="music-platform-load-more"
              :disabled="neteasePlaylistDetailLoading"
              @click="loadMoreNeteasePlaylistTracks"
            >
              {{ neteasePlaylistDetailLoading ? '加载中' : `加载更多（${neteasePlaylistLoadedLabel}）` }}
            </button>

            <div
              v-if="!neteasePlaylistDetailLoading && !neteasePlaylistDetail?.tracks.length"
              class="music-platform-empty"
            >
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
          <span>已支持搜索、读取歌单、在线歌词和临时在线播放；客户端或网页兜底后续单独接入。</span>
        </section>
      </div>

      <div v-else-if="activePanelView === 'kugou'" class="music-platform-panel">
        <section class="music-platform-card">
          <header class="music-platform-header">
            <div>
              <strong>酷狗音乐</strong>
              <span>{{ kugouStatusLabel }}</span>
            </div>
            <div class="music-platform-actions">
              <button type="button" :disabled="kugouLoginBusy" @click="refreshKugouLoginStatus(true)">
                刷新
              </button>
              <button
                v-if="!kugouLoggedIn"
                type="button"
                :disabled="kugouLoginBusy"
                @click="startKugouQrLogin"
              >
                扫码登录
              </button>
              <button v-else type="button" :disabled="kugouLoginBusy" @click="clearKugouLogin">
                退出
              </button>
            </div>
          </header>

          <div v-if="kugouProfile" class="music-platform-profile">
            <img
              v-if="kugouProfile.avatarUrl"
              :src="kugouProfile.avatarUrl"
              alt=""
              referrerpolicy="no-referrer"
            />
            <div v-else class="music-platform-avatar" aria-hidden="true">
              酷
            </div>
            <div class="music-platform-profile-main">
              <strong>{{ kugouProfile.nickname }}</strong>
              <span>用户 ID {{ kugouProfile.userId }}</span>
              <small v-if="kugouLoginStatus?.savedAt">
                本机保存于 {{ formatNeteaseTimestamp(kugouLoginStatus.savedAt) }}
              </small>
            </div>
            <span
              class="music-platform-membership"
              :class="{ active: kugouProfile.membership?.active }"
            >
              <span>{{ platformMembershipStatusLabel(kugouProfile.membership) }}</span>
              <small>{{ platformMembershipDetailLabel(kugouProfile.membership) }}</small>
            </span>
          </div>

          <div v-else class="music-platform-empty">
            <strong>{{ kugouLoggedIn ? '酷狗账号已连接' : '未连接酷狗账号' }}</strong>
            <span>{{ kugouStatusDetail }}</span>
          </div>

          <div v-if="kugouQrLogin" class="music-platform-qr">
            <img :src="kugouQrLogin.qrImage" alt="酷狗音乐登录二维码" />
            <div>
              <strong>{{ kugouStatusLabel }}</strong>
              <span>{{ kugouStatusDetail }}</span>
              <small>过期时间 {{ formatNeteaseTimestamp(kugouQrLogin.expiresAt) }}</small>
              <div class="music-platform-actions">
                <button type="button" :disabled="kugouLoginBusy" @click="startKugouQrLogin">
                  重新生成
                </button>
                <button type="button" @click="cancelKugouQrLogin">取消</button>
              </div>
            </div>
          </div>

          <p
            v-if="kugouLoginError || kugouLoginNotice"
            class="music-platform-message"
            :class="{ error: Boolean(kugouLoginError) }"
          >
            {{ kugouStatusDetail }}
          </p>

          <div class="music-platform-search-row">
            <label class="music-platform-search">
              <span>关键词</span>
              <input
                v-model="kugouSearchQuery"
                autocomplete="off"
                placeholder="搜索歌曲、歌手或专辑"
                @keydown.enter="searchKugouSongs()"
              />
            </label>
            <button type="button" :disabled="kugouSearchLoading" @click="searchKugouSongs()">
              {{ kugouSearchLoading ? '搜索中' : '搜索' }}
            </button>
          </div>

          <p
            v-if="kugouSearchError || kugouSearchNotice"
            class="music-platform-message"
            :class="{ error: Boolean(kugouSearchError) }"
          >
            {{ kugouSearchStatusDetail }}
          </p>

          <section class="music-platform-detail">
            <header class="music-platform-section-header">
              <div>
                <strong>搜索结果</strong>
                <span>
                  {{ kugouSearchLoadedLabel }}
                </span>
              </div>
              <span>临时列表</span>
            </header>

            <div v-if="kugouSearchLoading && kugouSearchTracks.length === 0" class="music-platform-empty">
              <strong>正在搜索酷狗音乐</strong>
              <span>搜索结果只保存在当前运行状态，不写入本机曲库。</span>
            </div>

            <div v-else-if="kugouSearchTracks.length === 0" class="music-platform-empty">
              <strong>暂无酷狗结果</strong>
              <span>输入关键词并点击“搜索”。</span>
            </div>

            <div v-else class="music-platform-track-list">
              <article
                v-for="(track, index) in kugouSearchTracks"
                :key="`${track.hash}-${index}`"
                class="music-platform-track-row"
                :class="{
                  active: currentTrack?.source === 'kugou' && currentTrack.kugouSongHash === track.hash,
                  unavailable: Boolean(kugouTrackUnavailableReason(track)),
                }"
              >
                <span class="music-platform-track-index">{{ index + 1 }}</span>
                <img
                  v-if="track.coverImgUrl"
                  :src="track.coverImgUrl"
                  alt=""
                  referrerpolicy="no-referrer"
                />
                <span v-else class="music-platform-track-cover" aria-hidden="true">酷</span>
                <span class="music-platform-track-copy">
                  <strong>{{ track.name }}</strong>
                  <small :title="formatKugouTrackSubline(track)">
                    {{ formatKugouTrackSubline(track) }}
                  </small>
                  <small
                    v-if="kugouTrackUnavailableReason(track)"
                    class="music-platform-track-warning"
                    :title="playbackFailureDetailTitle(kugouTrackUnavailableReason(track))"
                  >
                    不可播放：{{ compactPlaybackFailureReason(kugouTrackUnavailableReason(track)) }}
                  </small>
                </span>
                <span class="music-platform-track-actions">
                  <button
                    type="button"
                    :disabled="kugouLyricsLoading && kugouLyricsTrack?.hash === track.hash"
                    @click="showKugouLyrics(track)"
                  >
                    {{ kugouLyricsLoading && kugouLyricsTrack?.hash === track.hash ? '读取中' : '歌词' }}
                  </button>
                  <button
                    type="button"
                    :disabled="kugouTrackActionHash === track.hash"
                    @click="playKugouTrack(track, kugouSearchTracks)"
                  >
                    {{ kugouTrackActionHash === track.hash ? '获取中' : kugouTrackUnavailableReason(track) ? '重试' : '播放' }}
                  </button>
                </span>
              </article>
            </div>

            <button
              v-if="kugouSearchHasMore"
              type="button"
              class="music-platform-load-more"
              :disabled="kugouSearchLoading"
              @click="searchKugouSongs(true)"
            >
              {{ kugouSearchLoading ? '加载中' : `加载更多（${kugouSearchLoadedLabel}）` }}
            </button>
          </section>
        </section>

        <section class="music-platform-card">
          <header class="music-platform-section-header">
            <div>
              <strong>每日推荐</strong>
              <span>{{ kugouDailyRecommendationLoadedLabel }}</span>
            </div>
            <button
              type="button"
              :disabled="kugouDailyRecommendationLoading"
              @click="loadKugouDailyRecommendations(true)"
            >
              {{ kugouDailyRecommendationLoading ? '读取中' : '读取' }}
            </button>
          </header>

          <p
            v-if="kugouDailyRecommendationError || kugouDailyRecommendationNotice"
            class="music-platform-message"
            :class="{ error: Boolean(kugouDailyRecommendationError) }"
          >
            {{ kugouDailyRecommendationStatusDetail }}
          </p>

          <div
            v-if="kugouDailyRecommendationLoading && kugouDailyRecommendationTracks.length === 0"
            class="music-platform-empty"
          >
            <strong>正在读取酷狗每日推荐</strong>
            <span>歌曲摘要只保存在当前运行状态，不写入本机曲库。</span>
          </div>

          <div v-else-if="kugouDailyRecommendationTracks.length === 0" class="music-platform-empty">
            <strong>暂无每日推荐</strong>
            <span>点击读取获取酷狗每日推荐歌曲；部分账号可能需要登录。</span>
          </div>

          <div v-else class="music-platform-track-list">
            <article
              v-for="(track, index) in kugouDailyRecommendationTracks"
              :key="`kugou-daily-${track.hash}-${index}`"
              class="music-platform-track-row"
              :class="{
                active: currentTrack?.source === 'kugou' && currentTrack.kugouSongHash === track.hash,
                unavailable: Boolean(kugouTrackUnavailableReason(track)),
              }"
            >
              <span class="music-platform-track-index">{{ index + 1 }}</span>
              <img
                v-if="track.coverImgUrl"
                :src="track.coverImgUrl"
                alt=""
                referrerpolicy="no-referrer"
              />
              <span v-else class="music-platform-track-cover" aria-hidden="true">日</span>
              <span class="music-platform-track-copy">
                <strong>{{ track.name }}</strong>
                <small :title="formatKugouTrackSubline(track)">
                  {{ formatKugouTrackSubline(track) }}
                </small>
                <small
                  v-if="kugouTrackUnavailableReason(track)"
                  class="music-platform-track-warning"
                  :title="playbackFailureDetailTitle(kugouTrackUnavailableReason(track))"
                >
                  不可播放：{{ compactPlaybackFailureReason(kugouTrackUnavailableReason(track)) }}
                </small>
              </span>
              <span class="music-platform-track-actions">
                <button
                  type="button"
                  :disabled="kugouLyricsLoading && kugouLyricsTrack?.hash === track.hash"
                  @click="showKugouLyrics(track)"
                >
                  {{ kugouLyricsLoading && kugouLyricsTrack?.hash === track.hash ? '读取中' : '歌词' }}
                </button>
                <button
                  type="button"
                  :disabled="kugouTrackActionHash === track.hash"
                  @click="playKugouTrack(track, kugouDailyRecommendationTracks)"
                >
                  {{ kugouTrackActionHash === track.hash ? '获取中' : kugouTrackUnavailableReason(track) ? '重试' : '播放' }}
                </button>
              </span>
            </article>
          </div>
        </section>

        <section
          v-if="
            kugouLoggedIn ||
            kugouActivePlaylist ||
            kugouPlaylistDetail ||
            kugouPlaylistError ||
            kugouPlaylistsLoading ||
            kugouRecommendedPlaylistsLoading
          "
          class="music-platform-card"
        >
          <header class="music-platform-section-header">
            <div>
              <strong>{{ kugouActivePlaylist?.name || '当前歌单' }}</strong>
              <span>{{ kugouPlaylistStatusLabel }}</span>
            </div>
            <span v-if="kugouActivePlaylist">
              {{ kugouActivePlaylistSourceLabel }} · {{ kugouPlaylistLoadedLabel }}
            </span>
          </header>

          <p
            v-if="kugouPlaylistError || kugouPlaylistsLoading || kugouPlaylistDetail"
            class="music-platform-message"
            :class="{ error: Boolean(kugouPlaylistError) }"
          >
            {{ kugouPlaylistStatusLabel }}
          </p>

          <div
            v-if="
              (kugouPlaylistsLoading && kugouPlaylists.length === 0) ||
              (kugouRecommendedPlaylistsLoading && kugouRecommendedPlaylists.length === 0)
            "
            class="music-platform-empty"
          >
            <strong>{{ kugouRecommendedPlaylistsLoading ? '正在读取酷狗推荐歌单' : '正在读取酷狗个人歌单' }}</strong>
            <span>只读取歌单和歌曲摘要，不读取播放地址。</span>
          </div>

          <div v-else-if="!kugouActivePlaylist" class="music-platform-empty">
            <strong>暂无酷狗歌单数据</strong>
            <span>在左侧点击“刷新”读取我的歌单或推荐歌单。</span>
          </div>

          <section
            v-else-if="kugouActivePlaylist"
            ref="kugouPlaylistDetailSection"
            class="music-platform-detail"
          >
            <header class="music-platform-section-header">
              <div>
                <strong>{{ kugouActivePlaylist.name }}</strong>
                <span>{{ kugouPlaylistLoadedLabel }}</span>
              </div>
              <span v-if="kugouPlaylistHasMore">可继续加载</span>
            </header>

            <div
              v-if="kugouPlaylistDetailLoading && !kugouPlaylistDetail?.tracks.length"
              class="music-platform-empty"
            >
              <strong>正在读取歌曲摘要</strong>
              <span>读取完成后会显示歌曲、歌手、专辑和时长。</span>
            </div>

            <div v-if="kugouPlaylistDetail?.tracks.length" class="music-platform-track-list">
              <article
                v-for="(track, index) in kugouPlaylistDetail.tracks"
                :key="`kugou-playlist-${track.hash}-${index}`"
                class="music-platform-track-row"
                :class="{
                  active: currentTrack?.source === 'kugou' && currentTrack.kugouSongHash === track.hash,
                  unavailable: Boolean(kugouTrackUnavailableReason(track)),
                }"
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
                  <small :title="formatKugouTrackSubline(track)">
                    {{ formatKugouTrackSubline(track) }}
                  </small>
                  <small
                    v-if="kugouTrackUnavailableReason(track)"
                    class="music-platform-track-warning"
                    :title="playbackFailureDetailTitle(kugouTrackUnavailableReason(track))"
                  >
                    不可播放：{{ compactPlaybackFailureReason(kugouTrackUnavailableReason(track)) }}
                  </small>
                </span>
                <span class="music-platform-track-actions">
                  <button
                    type="button"
                    :disabled="kugouLyricsLoading && kugouLyricsTrack?.hash === track.hash"
                    @click="showKugouLyrics(track)"
                  >
                    {{ kugouLyricsLoading && kugouLyricsTrack?.hash === track.hash ? '读取中' : '歌词' }}
                  </button>
                  <button
                    type="button"
                    :disabled="kugouTrackActionHash === track.hash"
                    @click="playKugouTrack(track, kugouPlaylistDetail?.tracks ?? [])"
                  >
                    {{ kugouTrackActionHash === track.hash ? '获取中' : kugouTrackUnavailableReason(track) ? '重试' : '播放' }}
                  </button>
                </span>
              </article>
            </div>

            <button
              v-if="kugouPlaylistHasMore"
              type="button"
              class="music-platform-load-more"
              :disabled="kugouPlaylistDetailLoading"
              @click="loadMoreKugouPlaylistTracks"
            >
              {{ kugouPlaylistDetailLoading ? '加载中' : `加载更多（${kugouPlaylistLoadedLabel}）` }}
            </button>

            <div
              v-if="!kugouPlaylistDetailLoading && !kugouPlaylistDetail?.tracks.length"
              class="music-platform-empty"
            >
              <strong>暂无歌曲摘要</strong>
              <span>当前酷狗歌单没有返回可展示的歌曲信息。</span>
            </div>
          </section>
        </section>

        <section
          v-if="kugouLyricsTrack || kugouLyricsError"
          class="music-platform-card music-platform-lyrics-card"
        >
          <header class="music-platform-section-header">
            <div>
              <strong>{{ kugouLyricsTrack?.name || '酷狗歌词' }}</strong>
              <span>
                {{
                  kugouLyricsLoading
                    ? '正在读取在线歌词'
                    : kugouLyricsError || '酷狗在线歌词'
                }}
              </span>
            </div>
            <button type="button" @click="closeKugouLyrics">关闭</button>
          </header>

          <p
            v-if="kugouLyricsError || kugouLyricsResult?.warnings.length"
            class="music-platform-message"
            :class="{ error: Boolean(kugouLyricsError) }"
          >
            {{ kugouLyricsError || kugouLyricsResult?.warnings.join('；') }}
          </p>

          <div v-if="kugouLyricsLoading" class="music-platform-empty">
            <strong>正在读取酷狗歌词</strong>
            <span>读取完成后会显示 LRC 歌词，沉浸模式会自动同步使用。</span>
          </div>

          <div v-else-if="kugouLyricsResult" class="music-platform-lyrics-view">
            <p class="music-platform-message">
              已读取酷狗 LRC 歌词，沉浸模式会按时间轴同步显示。
            </p>
            <pre>{{ kugouLyricsResult.lrcContent || kugouLyricsResult.content }}</pre>
          </div>
        </section>

        <section class="music-platform-card is-muted">
          <strong>说明</strong>
          <span>酷狗本轮支持搜索、在线歌词和临时在线播放；播放地址为空时通常是版权、会员、地区或接口限制。</span>
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
              <small
                class="music-track-subline"
                :title="`${formatTrackListSubline(track)}\n${trackSourceTitle(track)}`"
              >
                {{ formatTrackListSubline(track) }}
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
      </section>
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
        aria-label="向歌单添加歌曲"
      >
        <header>
          <div>
            <h2>添加到歌单</h2>
            <p>{{ playlistTrackPickerTarget?.name || '选择歌曲' }}</p>
          </div>
          <button
            type="button"
            class="window-close"
            title="关闭添加歌曲"
            @click="closePlaylistTrackPicker"
          >
            ×
          </button>
        </header>

        <label class="music-field music-playlist-picker-search">
          搜索
          <input
            v-model="playlistTrackPickerQuery"
            autocomplete="off"
            placeholder="搜索歌名或歌手"
          />
        </label>

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
            <small>{{ trackArtistLabel(track) }} · {{ formatTrackDuration(track) }}</small>
          </label>
        </div>

        <div v-else class="music-empty">
          <strong>没有可添加的歌曲</strong>
          <span>当前歌单可能已经包含全部匹配歌曲。</span>
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

        <div class="music-quality-field" role="group" aria-label="在线播放音质">
          <span>{{ onlinePlaybackQualityPlatformLabel }}</span>
          <div class="music-quality-options">
            <button
              v-for="option in onlinePlaybackQualityOptions"
              :key="option.value"
              type="button"
              :class="{
                active: activeOnlinePlaybackQuality === option.value,
                unavailable: onlinePlaybackQualityOptionDisabled(option),
                unknown: option.availabilityStatus === 'unknown',
              }"
              :aria-pressed="activeOnlinePlaybackQuality === option.value"
              :title="onlinePlaybackQualityOptionTitle(option)"
              :disabled="onlinePlaybackQualitySwitching || onlinePlaybackQualityOptionDisabled(option)"
              @click="setOnlinePlaybackQuality(option.value)"
            >
              {{ option.label }}
            </button>
          </div>
          <small>{{ onlinePlaybackQualityStatusHint }}</small>
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
