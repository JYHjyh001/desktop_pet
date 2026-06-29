import { onBeforeUnmount, ref, shallowRef } from 'vue'

export interface MusicEnergyFrame {
  bass: number
  mid: number
  treble: number
  volume: number
  beat: number
}

type FrequencyDataArray = Uint8Array<ArrayBuffer>
type AudioContextConstructor = typeof AudioContext
type WebAudioWindow = Window &
  typeof globalThis & {
    webkitAudioContext?: AudioContextConstructor
  }
type CapturableMediaElement = HTMLMediaElement & {
  captureStream?: () => MediaStream
  mozCaptureStream?: () => MediaStream
  webkitCaptureStream?: () => MediaStream
}
type AudioAnalyzerSourceNode = MediaStreamAudioSourceNode | MediaElementAudioSourceNode

const SILENT_ENERGY_FRAME: MusicEnergyFrame = {
  bass: 0,
  mid: 0,
  treble: 0,
  volume: 0,
  beat: 0,
}

export function useMusicAudioAnalyzer() {
  const frequencyData = shallowRef<FrequencyDataArray | null>(null)
  const energyFrame = ref<MusicEnergyFrame>({ ...SILENT_ENERGY_FRAME })
  const analyzerReady = ref(false)
  const analyzerError = ref('')

  let audioContext: AudioContext | null = null
  let sourceNode: AudioAnalyzerSourceNode | null = null
  let analyserNode: AnalyserNode | null = null
  let connectedElement: HTMLMediaElement | null = null
  let mirrorElement: HTMLAudioElement | null = null
  let mirrorSourceElement: HTMLMediaElement | null = null
  let mirrorPlaybackErrorReported = false
  let animationFrameId: number | null = null
  let previousBass = 0

  function connectAudioElement(element: HTMLMediaElement | null) {
    if (!element || connectedElement === element) {
      return
    }

    try {
      ensureAudioGraph(element)
      startSampling()
    } catch (err) {
      analyzerReady.value = false
      analyzerError.value = `实时频谱不可用，已使用安全视觉回退：${String(err)}`
    }
  }

  async function resumeAnalyzer() {
    if (!audioContext || !analyserNode) {
      if (!connectedElement) {
        return
      }

      connectAudioElement(connectedElement)
    }

    if (!audioContext) {
      return
    }

    try {
      if (audioContext.state === 'suspended') {
        await audioContext.resume()
      }
      syncMirrorElement(true)
      startSampling()
    } catch (err) {
      analyzerReady.value = false
      analyzerError.value = `音乐可视化启动失败：${String(err)}`
    }
  }

  function resetEnergyFrame() {
    previousBass = 0
    energyFrame.value = { ...SILENT_ENERGY_FRAME }
  }

  function ensureAudioGraph(element: HTMLMediaElement) {
    if (sourceNode && connectedElement !== element) {
      throw new Error('当前播放器已经绑定到另一个音频元素')
    }

    if (sourceNode && analyserNode) {
      return
    }

    const AudioContextClass =
      window.AudioContext ?? (window as WebAudioWindow).webkitAudioContext
    if (!AudioContextClass) {
      throw new Error('当前系统 WebView 不支持 Web Audio')
    }

    audioContext = audioContext ?? new AudioContextClass()
    analyserNode = audioContext.createAnalyser()
    analyserNode.fftSize = 2048
    analyserNode.smoothingTimeConstant = 0.82

    sourceNode = createAnalyzerSource(audioContext, element, analyserNode)
    connectedElement = element
    frequencyData.value = new Uint8Array(analyserNode.frequencyBinCount)
    analyzerReady.value = true
    analyzerError.value = ''
  }

  function createAnalyzerSource(
    context: AudioContext,
    element: HTMLMediaElement,
    analyser: AnalyserNode,
  ): AudioAnalyzerSourceNode {
    const stream = captureElementStream(element)
    if (stream) {
      disposeMirrorElement()
      const streamSource = context.createMediaStreamSource(stream)
      streamSource.connect(analyser)
      return streamSource
    }

    const sourceUrl = element.currentSrc || element.src
    if (!sourceUrl) {
      throw new Error('当前音频地址不可用于实时频谱分析')
    }

    const mirror = new Audio()
    if (/^https?:/i.test(sourceUrl)) {
      mirror.crossOrigin = 'anonymous'
    }
    mirror.preload = 'auto'
    mirror.src = sourceUrl
    mirrorElement = mirror
    mirrorSourceElement = element
    mirrorPlaybackErrorReported = false

    const elementSource = context.createMediaElementSource(mirror)
    elementSource.connect(analyser)
    syncMirrorElement(true)
    return elementSource
  }

  function startSampling() {
    if (animationFrameId !== null || !analyserNode || !frequencyData.value) {
      return
    }

    const sample = () => {
      animationFrameId = window.requestAnimationFrame(sample)
      if (!analyserNode || !frequencyData.value) {
        return
      }

      syncMirrorElement()
      analyserNode.getByteFrequencyData(frequencyData.value)
      energyFrame.value = computeEnergyFrame(frequencyData.value, previousBass)
      previousBass = energyFrame.value.bass
    }

    sample()
  }

  function stopSampling() {
    if (animationFrameId !== null) {
      window.cancelAnimationFrame(animationFrameId)
      animationFrameId = null
    }
  }

  onBeforeUnmount(() => {
    stopSampling()
    sourceNode?.disconnect()
    analyserNode?.disconnect()
    disposeMirrorElement()
    void audioContext?.close()
  })

  function syncMirrorElement(force = false) {
    if (!mirrorElement || !mirrorSourceElement) {
      return
    }

    const sourceUrl = mirrorSourceElement.currentSrc || mirrorSourceElement.src
    if (sourceUrl && mirrorElement.src !== sourceUrl) {
      mirrorElement.pause()
      if (/^https?:/i.test(sourceUrl)) {
        mirrorElement.crossOrigin = 'anonymous'
      } else {
        mirrorElement.removeAttribute('crossorigin')
      }
      mirrorElement.src = sourceUrl
      mirrorElement.load()
      mirrorPlaybackErrorReported = false
    }

    if (mirrorElement.error) {
      reportMirrorAnalyzerError('当前音频地址不允许被 WebAudio 镜像读取')
      return
    }

    const sourceTime = mirrorSourceElement.currentTime
    if (
      Number.isFinite(sourceTime) &&
      (force || Math.abs(mirrorElement.currentTime - sourceTime) > 0.35)
    ) {
      try {
        mirrorElement.currentTime = sourceTime
      } catch {
        // Some streams cannot seek before metadata is ready.
      }
    }

    if (mirrorSourceElement.paused || mirrorSourceElement.ended) {
      if (!mirrorElement.paused) {
        mirrorElement.pause()
      }
      return
    }

    if (!mirrorElement.paused) {
      return
    }

    const playPromise = mirrorElement.play()
    if (!playPromise) {
      return
    }

    void playPromise
      .then(() => {
        mirrorPlaybackErrorReported = false
        if (analyserNode && frequencyData.value) {
          analyzerReady.value = true
          analyzerError.value = ''
        }
      })
      .catch(() => {
        reportMirrorAnalyzerError('当前 WebView 阻止镜像音频启动')
      })
  }

  function reportMirrorAnalyzerError(reason: string) {
    if (mirrorPlaybackErrorReported) {
      return
    }

    mirrorPlaybackErrorReported = true
    analyzerReady.value = false
    analyzerError.value = `实时频谱不可用，已使用安全视觉回退：${reason}`
    resetEnergyFrame()
  }

  function disposeMirrorElement() {
    if (!mirrorElement) {
      mirrorSourceElement = null
      mirrorPlaybackErrorReported = false
      return
    }

    mirrorElement.pause()
    mirrorElement.removeAttribute('src')
    mirrorElement.load()
    mirrorElement = null
    mirrorSourceElement = null
    mirrorPlaybackErrorReported = false
  }

  return {
    frequencyData,
    energyFrame,
    analyzerReady,
    analyzerError,
    connectAudioElement,
    resumeAnalyzer,
    resetEnergyFrame,
  }
}

function computeEnergyFrame(data: Uint8Array, previousBass: number): MusicEnergyFrame {
  const length = data.length
  if (length === 0) {
    return { ...SILENT_ENERGY_FRAME }
  }

  const bass = averageFrequencyRange(data, 0, Math.max(8, Math.floor(length * 0.08)))
  const mid = averageFrequencyRange(data, Math.floor(length * 0.08), Math.floor(length * 0.36))
  const treble = averageFrequencyRange(data, Math.floor(length * 0.36), length)
  const volume = bass * 0.45 + mid * 0.35 + treble * 0.2
  const beat = Math.max(0, bass - previousBass * 0.82) * 1.8

  return {
    bass: clamp01(bass),
    mid: clamp01(mid),
    treble: clamp01(treble),
    volume: clamp01(volume),
    beat: clamp01(beat),
  }
}

function averageFrequencyRange(data: Uint8Array, start: number, end: number) {
  const safeStart = Math.max(0, Math.min(data.length, start))
  const safeEnd = Math.max(safeStart + 1, Math.min(data.length, end))
  let total = 0

  for (let index = safeStart; index < safeEnd; index += 1) {
    total += data[index] ?? 0
  }

  return total / (safeEnd - safeStart) / 255
}

function clamp01(value: number) {
  if (!Number.isFinite(value)) {
    return 0
  }

  return Math.min(1, Math.max(0, value))
}

function captureElementStream(element: HTMLMediaElement) {
  const capturable = element as CapturableMediaElement
  const capture =
    capturable.captureStream ?? capturable.mozCaptureStream ?? capturable.webkitCaptureStream

  try {
    return capture ? capture.call(capturable) : null
  } catch {
    return null
  }
}
