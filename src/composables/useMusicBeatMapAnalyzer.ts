import { computed, onBeforeUnmount, ref, shallowRef } from 'vue'
import type { MusicEnergyFrame } from './useMusicAudioAnalyzer'

export interface MusicBeatMapTrack {
  id: string
  url: string
  duration: number | null
}

export interface MusicBeatFrame extends MusicEnergyFrame {
  time: number
}

export interface MusicBeatMap {
  trackId: string
  duration: number
  frameStep: number
  frames: MusicBeatFrame[]
  analyzedAt: number
}

type BeatMapStatus = 'idle' | 'analyzing' | 'ready' | 'error'
type AudioContextConstructor = typeof AudioContext
type WebAudioWindow = Window &
  typeof globalThis & {
    webkitAudioContext?: AudioContextConstructor
  }

const FRAME_STEP_SECONDS = 0.05
const TARGET_ANALYSIS_SAMPLE_RATE = 12000

export function useMusicBeatMapAnalyzer() {
  const beatMap = shallowRef<MusicBeatMap | null>(null)
  const beatMapStatus = ref<BeatMapStatus>('idle')
  const beatMapError = ref('')
  const beatMapProgress = ref(0)

  let activeAbortController: AbortController | null = null

  const beatMapReady = computed(() => beatMapStatus.value === 'ready' && Boolean(beatMap.value))

  async function analyzeTrack(track: MusicBeatMapTrack) {
    activeAbortController?.abort()

    if (!track.url) {
      resetBeatMap()
      return
    }

    const abortController = new AbortController()
    activeAbortController = abortController
    beatMapStatus.value = 'analyzing'
    beatMapError.value = ''
    beatMapProgress.value = 0

    try {
      const map = await buildBeatMap(track, abortController.signal, (progress) => {
        if (activeAbortController === abortController) {
          beatMapProgress.value = progress
        }
      })

      if (activeAbortController !== abortController || abortController.signal.aborted) {
        return
      }

      beatMap.value = map
      beatMapStatus.value = 'ready'
      beatMapProgress.value = 1
    } catch (err) {
      if (abortController.signal.aborted) {
        return
      }

      beatMap.value = null
      beatMapStatus.value = 'error'
      beatMapError.value = `离线节奏分析失败：${formatError(err)}`
      beatMapProgress.value = 0
    } finally {
      if (activeAbortController === abortController) {
        activeAbortController = null
      }
    }
  }

  function resetBeatMap() {
    activeAbortController?.abort()
    activeAbortController = null
    beatMap.value = null
    beatMapStatus.value = 'idle'
    beatMapError.value = ''
    beatMapProgress.value = 0
  }

  function frameAt(time: number): MusicEnergyFrame | null {
    const map = beatMap.value
    if (!map || map.frames.length === 0) {
      return null
    }

    const safeTime = clamp(time, 0, map.duration || Number.POSITIVE_INFINITY)
    const exactIndex = safeTime / map.frameStep
    const leftIndex = clampIndex(Math.floor(exactIndex), map.frames.length)
    const rightIndex = clampIndex(leftIndex + 1, map.frames.length)
    const left = map.frames[leftIndex]
    const right = map.frames[rightIndex]

    if (!left || !right || leftIndex === rightIndex) {
      return left ?? right ?? null
    }

    return interpolateFrame(left, right, exactIndex - leftIndex)
  }

  function frequencyDataAt(time: number): Uint8Array | null {
    const frame = frameAt(time)
    if (!frame) {
      return null
    }

    return synthesizeFrequencyData(frame, time)
  }

  onBeforeUnmount(() => {
    activeAbortController?.abort()
  })

  return {
    beatMap,
    beatMapStatus,
    beatMapError,
    beatMapProgress,
    beatMapReady,
    analyzeTrack,
    frameAt,
    frequencyDataAt,
    resetBeatMap,
  }
}

async function buildBeatMap(
  track: MusicBeatMapTrack,
  signal: AbortSignal,
  onProgress: (progress: number) => void,
): Promise<MusicBeatMap> {
  onProgress(0.04)
  const response = await fetch(track.url, { signal })

  if (!response.ok) {
    throw new Error(`无法读取音频数据（HTTP ${response.status}）`)
  }

  onProgress(0.18)
  const audioBytes = await response.arrayBuffer()
  throwIfAborted(signal)

  const AudioContextClass =
    window.AudioContext ?? (window as WebAudioWindow).webkitAudioContext
  if (!AudioContextClass) {
    throw new Error('当前系统 WebView 不支持本机音频解码')
  }

  const audioContext = new AudioContextClass()
  try {
    onProgress(0.34)
    const audioBuffer = await audioContext.decodeAudioData(audioBytes.slice(0))
    throwIfAborted(signal)
    onProgress(0.66)
    const frames = analyzeAudioBuffer(audioBuffer, FRAME_STEP_SECONDS)
    onProgress(0.92)

    return {
      trackId: track.id,
      duration: audioBuffer.duration || sanitizeDuration(track.duration),
      frameStep: FRAME_STEP_SECONDS,
      frames,
      analyzedAt: Date.now(),
    }
  } finally {
    void audioContext.close()
  }
}

function analyzeAudioBuffer(audioBuffer: AudioBuffer, frameStep: number): MusicBeatFrame[] {
  const sampleRate = audioBuffer.sampleRate
  const samplesPerFrame = Math.max(256, Math.floor(sampleRate * frameStep))
  const sampleStride = Math.max(1, Math.floor(sampleRate / TARGET_ANALYSIS_SAMPLE_RATE))
  const frameCount = Math.max(1, Math.ceil(audioBuffer.length / samplesPerFrame))
  const channels = collectChannels(audioBuffer)
  const rawFrames: Array<{
    time: number
    bass: number
    mid: number
    treble: number
    volume: number
  }> = []

  let previousSample = 0
  let previousDiff = 0
  let lowPass = 0

  for (let frameIndex = 0; frameIndex < frameCount; frameIndex += 1) {
    const start = frameIndex * samplesPerFrame
    const end = Math.min(audioBuffer.length, start + samplesPerFrame)
    let lowSum = 0
    let volumeSum = 0
    let midSum = 0
    let trebleSum = 0
    let count = 0

    for (let sampleIndex = start; sampleIndex < end; sampleIndex += sampleStride) {
      const sample = averageChannelSample(channels, sampleIndex)
      lowPass += (sample - lowPass) * 0.035
      const diff = sample - previousSample
      const snap = diff - previousDiff

      lowSum += lowPass * lowPass
      volumeSum += sample * sample
      midSum += diff * diff
      trebleSum += snap * snap
      previousSample = sample
      previousDiff = diff
      count += 1
    }

    const safeCount = Math.max(1, count)
    rawFrames.push({
      time: start / sampleRate,
      bass: Math.sqrt(lowSum / safeCount),
      mid: Math.sqrt(midSum / safeCount),
      treble: Math.sqrt(trebleSum / safeCount),
      volume: Math.sqrt(volumeSum / safeCount),
    })
  }

  return normalizeFrames(rawFrames)
}

function normalizeFrames(
  rawFrames: Array<{
    time: number
    bass: number
    mid: number
    treble: number
    volume: number
  }>,
): MusicBeatFrame[] {
  const bassScale = percentile(rawFrames.map((frame) => frame.bass), 0.94)
  const midScale = percentile(rawFrames.map((frame) => frame.mid), 0.94)
  const trebleScale = percentile(rawFrames.map((frame) => frame.treble), 0.94)
  const volumeScale = percentile(rawFrames.map((frame) => frame.volume), 0.94)
  let previousBass = 0
  let previousVolume = 0

  return rawFrames.map((frame) => {
    const bass = soften(frame.bass / bassScale)
    const mid = soften(frame.mid / midScale)
    const treble = soften(frame.treble / trebleScale)
    const volume = soften(frame.volume / volumeScale)
    const bassOnset = Math.max(0, bass - previousBass * 0.88)
    const volumeOnset = Math.max(0, volume - previousVolume * 0.94)
    const beat = clamp(bassOnset * 1.7 + volumeOnset * 0.8, 0, 1)

    previousBass = bass
    previousVolume = volume

    return {
      time: frame.time,
      bass,
      mid,
      treble,
      volume,
      beat,
    }
  })
}

function synthesizeFrequencyData(frame: MusicEnergyFrame, time: number) {
  const data = new Uint8Array(128)

  for (let index = 0; index < data.length; index += 1) {
    const progress = index / Math.max(1, data.length - 1)
    const bandEnergy =
      progress < 0.16 ? frame.bass : progress < 0.58 ? frame.mid : frame.treble
    const ripple =
      Math.sin(index * 0.43 + time * 5.4) * 0.08 +
      Math.sin(index * 0.17 - time * 2.8) * 0.05
    const slope = 1 - progress * 0.34
    const value = clamp(
      (bandEnergy * slope + frame.volume * 0.16 + frame.beat * 0.18 + ripple) * 255,
      0,
      255,
    )
    data[index] = Math.round(value)
  }

  return data
}

function interpolateFrame(left: MusicBeatFrame, right: MusicBeatFrame, amount: number): MusicEnergyFrame {
  const progress = easeFrameProgress(clamp(amount, 0, 1))

  return {
    bass: lerp(left.bass, right.bass, progress),
    mid: lerp(left.mid, right.mid, progress),
    treble: lerp(left.treble, right.treble, progress),
    volume: lerp(left.volume, right.volume, progress),
    beat: lerp(left.beat, right.beat, progress),
  }
}

function easeFrameProgress(value: number) {
  return value * value * (3 - 2 * value)
}

function lerp(left: number, right: number, amount: number) {
  return left + (right - left) * amount
}

function collectChannels(audioBuffer: AudioBuffer) {
  const channels: Float32Array[] = []
  const count = Math.max(1, audioBuffer.numberOfChannels)

  for (let index = 0; index < count; index += 1) {
    channels.push(audioBuffer.getChannelData(index))
  }

  return channels
}

function averageChannelSample(channels: Float32Array[], sampleIndex: number) {
  let total = 0

  for (const channel of channels) {
    total += channel[sampleIndex] ?? 0
  }

  return total / Math.max(1, channels.length)
}

function percentile(values: number[], ratio: number) {
  if (values.length === 0) {
    return 1
  }

  const sorted = [...values].sort((left, right) => left - right)
  const index = clampIndex(Math.floor(sorted.length * ratio), sorted.length)
  return Math.max(0.000001, sorted[index] ?? 1)
}

function soften(value: number) {
  return clamp(Math.pow(clamp(value, 0, 1.8), 0.74) / Math.pow(1.8, 0.74), 0, 1)
}

function sanitizeDuration(value: number | null) {
  return typeof value === 'number' && Number.isFinite(value) && value > 0 ? value : 0
}

function clamp(value: number, min: number, max: number) {
  if (!Number.isFinite(value)) {
    return min
  }

  return Math.min(max, Math.max(min, value))
}

function clampIndex(index: number, length: number) {
  if (length <= 0) {
    return 0
  }

  return Math.min(length - 1, Math.max(0, index))
}

function formatError(err: unknown) {
  return err instanceof Error ? err.message : String(err)
}

function throwIfAborted(signal: AbortSignal) {
  if (signal.aborted) {
    throw new DOMException('Aborted', 'AbortError')
  }
}
