<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import type { DrawerTheme } from '../types/app'
import type { MusicEnergyFrame } from '../composables/useMusicAudioAnalyzer'

type MusicVisualMode = 'rhythm' | 'dance' | 'focus' | 'sleep'

const props = withDefaults(
  defineProps<{
    frequencyData: Uint8Array | null
    energy: MusicEnergyFrame
    playing: boolean
    mode: MusicVisualMode
    intensity: number
    reducedMotion: boolean
    theme: DrawerTheme | string
  }>(),
  {
    frequencyData: null,
    mode: 'rhythm',
    intensity: 0.72,
    reducedMotion: false,
    theme: 'light',
  },
)

const canvas = ref<HTMLCanvasElement | null>(null)

let animationFrameId: number | null = null
let resizeObserver: ResizeObserver | null = null
let canvasWidth = 1
let canvasHeight = 1

onMounted(() => {
  const element = canvas.value
  if (!element) {
    return
  }

  resizeObserver = new ResizeObserver(() => resizeCanvas())
  resizeObserver.observe(element)
  resizeCanvas()
  render(0)
})

onBeforeUnmount(() => {
  if (animationFrameId !== null) {
    window.cancelAnimationFrame(animationFrameId)
    animationFrameId = null
  }
  resizeObserver?.disconnect()
})

function resizeCanvas() {
  const element = canvas.value
  if (!element) {
    return
  }

  const rect = element.getBoundingClientRect()
  const ratio = window.devicePixelRatio || 1
  canvasWidth = Math.max(1, Math.floor(rect.width * ratio))
  canvasHeight = Math.max(1, Math.floor(rect.height * ratio))

  if (element.width !== canvasWidth || element.height !== canvasHeight) {
    element.width = canvasWidth
    element.height = canvasHeight
  }
}

function render(time: number) {
  animationFrameId = window.requestAnimationFrame(render)

  const element = canvas.value
  const context = element?.getContext('2d')
  if (!element || !context) {
    return
  }

  resizeCanvas()
  const palette = visualPalette(props.theme, props.mode)
  const energy = props.energy
  const intensity = clamp(props.intensity, 0.2, 1)
  const motion = props.reducedMotion ? 0.26 : 1
  const playFactor = props.playing ? 1 : 0.34
  const pulse = (energy.beat * 0.65 + energy.volume * 0.35) * intensity * playFactor

  context.clearRect(0, 0, canvasWidth, canvasHeight)
  drawBackdrop(context, palette)
  drawGrid(context, palette, time, motion)
  drawWave(context, palette, time, intensity, motion, playFactor)
  drawSpectrum(context, palette, time, intensity, motion, playFactor)
  drawPulse(context, palette, pulse, time, motion)
}

function drawBackdrop(context: CanvasRenderingContext2D, palette: VisualPalette) {
  const gradient = context.createLinearGradient(0, 0, canvasWidth, canvasHeight)
  gradient.addColorStop(0, palette.backgroundStart)
  gradient.addColorStop(0.55, palette.backgroundMiddle)
  gradient.addColorStop(1, palette.backgroundEnd)
  context.fillStyle = gradient
  context.fillRect(0, 0, canvasWidth, canvasHeight)
}

function drawGrid(
  context: CanvasRenderingContext2D,
  palette: VisualPalette,
  time: number,
  motion: number,
) {
  const gap = Math.max(26, Math.floor(canvasWidth / 28))
  const drift = ((time * 0.018 * motion) % gap) - gap

  context.save()
  context.globalAlpha = palette.gridAlpha
  context.strokeStyle = palette.grid
  context.lineWidth = 1

  for (let x = drift; x < canvasWidth + gap; x += gap) {
    context.beginPath()
    context.moveTo(x, 0)
    context.lineTo(x + canvasWidth * 0.08, canvasHeight)
    context.stroke()
  }

  for (let y = canvasHeight * 0.18; y < canvasHeight; y += gap) {
    context.beginPath()
    context.moveTo(0, y)
    context.lineTo(canvasWidth, y + canvasHeight * 0.04)
    context.stroke()
  }

  context.restore()
}

function drawSpectrum(
  context: CanvasRenderingContext2D,
  palette: VisualPalette,
  time: number,
  intensity: number,
  motion: number,
  playFactor: number,
) {
  const data = props.frequencyData
  const barCount = 72
  const width = canvasWidth / barCount
  const baseline = canvasHeight * 0.74
  const maxHeight = canvasHeight * (props.mode === 'focus' ? 0.24 : 0.44)

  context.save()
  context.lineCap = 'round'

  for (let index = 0; index < barCount; index += 1) {
    const ratio = frequencyRatio(data, index, barCount, time, motion)
    const shaped = Math.pow(ratio, 0.74)
    const barHeight = Math.max(5, shaped * maxHeight * intensity * playFactor)
    const x = index * width + width * 0.24
    const y = baseline - barHeight
    const gradient = context.createLinearGradient(0, y, 0, baseline + barHeight * 0.42)

    gradient.addColorStop(0, palette.high)
    gradient.addColorStop(0.48, palette.mid)
    gradient.addColorStop(1, palette.low)
    context.fillStyle = gradient
    context.globalAlpha = 0.42 + shaped * 0.5
    roundRect(context, x, y, Math.max(2, width * 0.48), barHeight, Math.max(2, width * 0.24))
    context.fill()

    context.globalAlpha = 0.15 + shaped * 0.2
    roundRect(
      context,
      x,
      baseline + 5,
      Math.max(2, width * 0.48),
      barHeight * 0.32,
      Math.max(2, width * 0.24),
    )
    context.fill()
  }

  context.restore()
}

function drawWave(
  context: CanvasRenderingContext2D,
  palette: VisualPalette,
  time: number,
  intensity: number,
  motion: number,
  playFactor: number,
) {
  const centerY = canvasHeight * 0.45
  const amplitude =
    canvasHeight *
    (0.035 + props.energy.mid * 0.12 + props.energy.treble * 0.08) *
    intensity *
    playFactor
  const phase = time * 0.0018 * motion

  context.save()
  context.lineWidth = Math.max(2, canvasHeight * 0.006)
  context.strokeStyle = palette.wave
  context.globalAlpha = props.mode === 'sleep' ? 0.42 : 0.62
  context.beginPath()

  for (let x = 0; x <= canvasWidth; x += Math.max(6, canvasWidth / 150)) {
    const progress = x / canvasWidth
    const wave =
      Math.sin(progress * Math.PI * 4 + phase) * amplitude +
      Math.sin(progress * Math.PI * 9 - phase * 0.72) * amplitude * 0.38
    const y = centerY + wave

    if (x === 0) {
      context.moveTo(x, y)
    } else {
      context.lineTo(x, y)
    }
  }

  context.stroke()
  context.restore()
}

function drawPulse(
  context: CanvasRenderingContext2D,
  palette: VisualPalette,
  pulse: number,
  time: number,
  motion: number,
) {
  const centerX = canvasWidth * 0.5
  const centerY = canvasHeight * 0.5
  const maxRadius = Math.min(canvasWidth, canvasHeight) * 0.42
  const base = Math.min(canvasWidth, canvasHeight) * 0.18
  const rings = props.reducedMotion ? 2 : 4

  context.save()
  context.lineWidth = Math.max(1.5, canvasHeight * 0.004)

  for (let index = 0; index < rings; index += 1) {
    const offset = ((time * 0.00012 * motion + index / rings) % 1) * maxRadius
    const radius = base + offset + pulse * maxRadius * 0.18
    const alpha = Math.max(0, 0.28 - offset / maxRadius * 0.22) + pulse * 0.18
    context.globalAlpha = alpha
    context.strokeStyle = index % 2 === 0 ? palette.pulse : palette.wave
    context.beginPath()
    context.ellipse(centerX, centerY, radius * 1.34, radius * 0.62, 0, 0, Math.PI * 2)
    context.stroke()
  }

  context.restore()
}

function frequencyRatio(
  data: Uint8Array | null,
  index: number,
  count: number,
  time: number,
  motion: number,
) {
  if (!data || data.length === 0) {
    return fallbackFrequencyRatio(index, count, time, motion)
  }

  const dataIndex = Math.min(data.length - 1, Math.floor((index / count) ** 1.55 * data.length))
  const ratio = (data[dataIndex] ?? 0) / 255
  const beatRipple = props.energy.beat * Math.max(0, Math.sin(time * 0.018 * motion + index * 0.5)) * 0.08
  return clamp(ratio + beatRipple, 0, 1)
}

function fallbackFrequencyRatio(index: number, count: number, time: number, motion: number) {
  const progress = index / Math.max(1, count - 1)
  const band =
    progress < 0.16 ? props.energy.bass : progress < 0.58 ? props.energy.mid : props.energy.treble
  const wave =
    Math.sin(time * 0.008 * motion + index * 0.46) * 0.08 +
    Math.sin(time * 0.0035 * motion - index * 0.31) * 0.05
  const pulse = props.energy.volume * 0.2 + props.energy.beat * 0.24
  const idleMotion = props.playing ? 0.1 : 0.035

  return clamp(0.12 + idleMotion + band * 0.62 + pulse + wave, 0.06, 1)
}

function roundRect(
  context: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  radius: number,
) {
  const safeRadius = Math.min(radius, width / 2, Math.abs(height) / 2)
  context.beginPath()
  context.moveTo(x + safeRadius, y)
  context.lineTo(x + width - safeRadius, y)
  context.quadraticCurveTo(x + width, y, x + width, y + safeRadius)
  context.lineTo(x + width, y + height - safeRadius)
  context.quadraticCurveTo(x + width, y + height, x + width - safeRadius, y + height)
  context.lineTo(x + safeRadius, y + height)
  context.quadraticCurveTo(x, y + height, x, y + height - safeRadius)
  context.lineTo(x, y + safeRadius)
  context.quadraticCurveTo(x, y, x + safeRadius, y)
}

function visualPalette(theme: DrawerTheme | string, mode: MusicVisualMode): VisualPalette {
  if (theme === 'animal-island') {
    return {
      backgroundStart: mode === 'sleep' ? '#2c3140' : '#244f54',
      backgroundMiddle: mode === 'focus' ? '#5a7f71' : '#2d756b',
      backgroundEnd: '#f4bd77',
      grid: '#fff7df',
      gridAlpha: 0.12,
      low: '#ffd97b',
      mid: '#7fb6a6',
      high: '#fff8e9',
      wave: '#ffe7ad',
      pulse: '#bfe8d0',
    }
  }

  if (mode === 'sleep') {
    return {
      backgroundStart: '#111827',
      backgroundMiddle: '#263754',
      backgroundEnd: '#4b5e7b',
      grid: '#d7e4ff',
      gridAlpha: 0.08,
      low: '#86b7ff',
      mid: '#b5c7f1',
      high: '#ffffff',
      wave: '#b8cdfa',
      pulse: '#8fb3ff',
    }
  }

  if (mode === 'focus') {
    return {
      backgroundStart: '#102424',
      backgroundMiddle: '#1f6f70',
      backgroundEnd: '#527870',
      grid: '#d9fff2',
      gridAlpha: 0.1,
      low: '#b6e6cf',
      mid: '#78d3c7',
      high: '#f5fff9',
      wave: '#b6e6cf',
      pulse: '#78d3c7',
    }
  }

  return {
    backgroundStart: '#10192a',
    backgroundMiddle: '#1f6f70',
    backgroundEnd: mode === 'dance' ? '#a35b2c' : '#263754',
    grid: '#d8f8ff',
    gridAlpha: 0.1,
    low: '#ffb861',
    mid: '#2aa7d9',
    high: '#f9fcff',
    wave: '#b8f2ff',
    pulse: '#ffcf7a',
  }
}

function clamp(value: number, min: number, max: number) {
  if (!Number.isFinite(value)) {
    return min
  }

  return Math.min(max, Math.max(min, value))
}

interface VisualPalette {
  backgroundStart: string
  backgroundMiddle: string
  backgroundEnd: string
  grid: string
  gridAlpha: number
  low: string
  mid: string
  high: string
  wave: string
  pulse: string
}
</script>

<template>
  <canvas ref="canvas" class="music-visualizer-canvas" aria-hidden="true" />
</template>
