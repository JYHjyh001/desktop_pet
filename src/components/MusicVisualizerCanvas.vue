<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import type { MusicImmersiveTheme, MusicLineStyle, MusicRippleStyle, MusicSpectrumStyle } from '../types/app'
import type { MusicEnergyFrame } from '../composables/useMusicAudioAnalyzer'

type MusicVisualMode = 'rhythm' | 'dance' | 'focus' | 'sleep'
const TAU = Math.PI * 2

const props = withDefaults(
  defineProps<{
    frequencyData: Uint8Array | null
    energy: MusicEnergyFrame
    playing: boolean
    mode: MusicVisualMode
    spectrumStyle: MusicSpectrumStyle
    lineStyle: MusicLineStyle
    rippleStyle: MusicRippleStyle
    intensity: number
    reducedMotion: boolean
    theme: MusicImmersiveTheme | string
    disableForeground: boolean
  }>(),
  {
    frequencyData: null,
    mode: 'rhythm',
    spectrumStyle: 'bars',
    lineStyle: 'wave',
    rippleStyle: 'rings',
    intensity: 0.72,
    reducedMotion: false,
    theme: 'light',
    disableForeground: false,
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
  const foregroundDisabled =
    props.disableForeground ||
    (props.spectrumStyle === 'none' && props.lineStyle === 'none' && props.rippleStyle === 'none')

  context.clearRect(0, 0, canvasWidth, canvasHeight)
  drawBackdrop(context, palette)
  if (foregroundDisabled) {
    return
  }
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

function drawSpectrum(
  context: CanvasRenderingContext2D,
  palette: VisualPalette,
  time: number,
  intensity: number,
  motion: number,
  playFactor: number,
) {
  if (props.spectrumStyle === 'none') {
    return
  }

  if (props.spectrumStyle === 'mirror') {
    drawMirrorSpectrum(context, palette, time, intensity, motion, playFactor)
    return
  }

  if (props.spectrumStyle === 'orbit') {
    drawOrbitSpectrum(context, palette, time, intensity, motion, playFactor)
    return
  }

  if (props.spectrumStyle === 'particles') {
    drawParticleSpectrum(context, palette, time, intensity, motion, playFactor)
    return
  }

  if (props.spectrumStyle === 'ribbon') {
    drawRibbonSpectrum(context, palette, time, intensity, motion, playFactor)
    return
  }

  drawBarSpectrum(context, palette, time, intensity, motion, playFactor)
}

function drawBarSpectrum(
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

function drawMirrorSpectrum(
  context: CanvasRenderingContext2D,
  palette: VisualPalette,
  time: number,
  intensity: number,
  motion: number,
  playFactor: number,
) {
  const data = props.frequencyData
  const barCount = props.reducedMotion ? 52 : 68
  const width = canvasWidth / barCount
  const centerY = canvasHeight * (props.mode === 'sleep' ? 0.6 : 0.62)
  const maxHeight = canvasHeight * (props.mode === 'focus' ? 0.16 : 0.3)

  context.save()
  context.globalCompositeOperation = 'lighter'

  const spineGradient = context.createLinearGradient(0, 0, canvasWidth, 0)
  spineGradient.addColorStop(0, 'transparent')
  spineGradient.addColorStop(0.28, palette.wave)
  spineGradient.addColorStop(0.5, palette.high)
  spineGradient.addColorStop(0.72, palette.wave)
  spineGradient.addColorStop(1, 'transparent')
  context.globalAlpha = 0.12 + props.energy.beat * 0.18 * playFactor
  context.fillStyle = spineGradient
  context.fillRect(0, centerY - 1.5, canvasWidth, 3)

  for (let index = 0; index < barCount; index += 1) {
    const ratio = frequencyRatio(data, index, barCount, time, motion)
    const shaped = Math.pow(ratio, 0.68)
    const beatLift = 1 + props.energy.bass * 0.16
    const barHeight = Math.max(4, shaped * maxHeight * intensity * playFactor * beatLift)
    const x = index * width + width * 0.18
    const barWidth = Math.max(2, width * 0.58)
    const radius = Math.max(2, barWidth * 0.45)
    const gradient = context.createLinearGradient(0, centerY - barHeight, 0, centerY + barHeight)

    gradient.addColorStop(0, palette.high)
    gradient.addColorStop(0.5, palette.mid)
    gradient.addColorStop(1, palette.low)
    context.fillStyle = gradient
    context.globalAlpha = 0.22 + shaped * 0.52
    context.shadowColor = palette.pulse
    context.shadowBlur = shaped * 14
    roundRect(context, x, centerY - barHeight, barWidth, barHeight, radius)
    context.fill()

    context.globalAlpha = 0.12 + shaped * 0.34
    context.shadowBlur = shaped * 10
    roundRect(context, x, centerY + 5, barWidth, barHeight * 0.72, radius)
    context.fill()
  }

  context.restore()
}

function drawOrbitSpectrum(
  context: CanvasRenderingContext2D,
  palette: VisualPalette,
  time: number,
  intensity: number,
  motion: number,
  playFactor: number,
) {
  const data = props.frequencyData
  const tickCount = props.reducedMotion ? 72 : 104
  const centerX = canvasWidth * 0.5
  const centerY = canvasHeight * (props.mode === 'sleep' ? 0.52 : 0.5)
  const baseRadius = Math.min(canvasWidth, canvasHeight) * (props.mode === 'focus' ? 0.15 : 0.18)
  const maxLift = Math.min(canvasWidth, canvasHeight) * 0.18 * intensity * playFactor
  const rotation = (props.reducedMotion ? -0.25 : time * 0.00014 * motion) - Math.PI / 2
  const beatRadius = props.energy.beat * maxLift * 0.1

  context.save()
  context.globalCompositeOperation = 'lighter'
  context.lineCap = 'round'

  for (let ring = 0; ring < 2; ring += 1) {
    context.globalAlpha = 0.08 + props.energy.volume * 0.12
    context.strokeStyle = ring === 0 ? palette.wave : palette.pulse
    context.lineWidth = Math.max(1, canvasHeight * 0.0025)
    context.beginPath()
    context.ellipse(
      centerX,
      centerY,
      baseRadius + beatRadius + ring * maxLift * 0.36,
      (baseRadius + beatRadius + ring * maxLift * 0.36) * 0.72,
      0,
      0,
      TAU,
    )
    context.stroke()
  }

  for (let index = 0; index < tickCount; index += 1) {
    const progress = index / tickCount
    const angle = rotation + progress * TAU
    const ratio = frequencyRatio(data, index, tickCount, time, motion)
    const shaped = Math.pow(ratio, 0.72)
    const inner = baseRadius + beatRadius + Math.sin(time * 0.0012 * motion + index) * 2 * motion
    const outer = inner + 4 + shaped * maxLift
    const squash = 0.72
    const startX = centerX + Math.cos(angle) * inner
    const startY = centerY + Math.sin(angle) * inner * squash
    const endX = centerX + Math.cos(angle) * outer
    const endY = centerY + Math.sin(angle) * outer * squash

    context.globalAlpha = 0.16 + shaped * 0.64
    context.strokeStyle = progress < 0.34 ? palette.low : progress < 0.68 ? palette.mid : palette.high
    context.lineWidth = Math.max(1.2, 1.4 + shaped * 4.2)
    context.shadowColor = progress < 0.5 ? palette.pulse : palette.wave
    context.shadowBlur = shaped * 12
    context.beginPath()
    context.moveTo(startX, startY)
    context.lineTo(endX, endY)
    context.stroke()

    if (shaped > 0.58 && index % 4 === 0) {
      context.globalAlpha = Math.min(0.72, shaped)
      context.fillStyle = palette.high
      context.beginPath()
      context.arc(endX, endY, 1.2 + shaped * 3.2, 0, TAU)
      context.fill()
    }
  }

  context.restore()
}

function drawParticleSpectrum(
  context: CanvasRenderingContext2D,
  palette: VisualPalette,
  time: number,
  intensity: number,
  motion: number,
  playFactor: number,
) {
  const data = props.frequencyData
  const particleCount = props.reducedMotion ? 52 : 92
  const centerX = canvasWidth * 0.5
  const centerY = canvasHeight * 0.5
  const maxRadius = Math.min(canvasWidth, canvasHeight) * 0.52
  const driftScale = props.reducedMotion ? 0.28 : motion
  const beatLift = props.energy.beat * playFactor
  const bassLift = props.energy.bass * playFactor
  const volumeLift = props.energy.volume * playFactor
  const reactionLift = clamp(0.2 + beatLift * 0.72 + bassLift * 0.48 + volumeLift * 0.36, 0, 1.45)

  context.save()
  context.globalCompositeOperation = 'lighter'

  const cloudCount = props.reducedMotion ? 2 : 3
  for (let index = 0; index < cloudCount; index += 1) {
    const seedA = pseudoRandom(index * 41 + 13)
    const seedB = pseudoRandom(index * 43 + 17)
    const ratio = Math.pow(frequencyRatio(data, index * 13, cloudCount * 18, time, motion), 0.6)
    const angle = seedA * TAU + time * 0.00008 * driftScale * (seedB > 0.5 ? 1 : -1)
    const distance = maxRadius * (0.18 + seedB * 0.48) * (0.95 + bassLift * 0.12)
    const x = centerX + Math.cos(angle) * distance
    const y = centerY + Math.sin(angle) * distance * 0.62
    const radius = maxRadius * (0.12 + ratio * 0.12 + reactionLift * 0.04)
    const color = index % 3 === 0 ? palette.low : index % 3 === 1 ? palette.mid : palette.pulse
    const gradient = context.createRadialGradient(x, y, 0, x, y, radius)

    gradient.addColorStop(0, colorWithAlpha(color, 0.34))
    gradient.addColorStop(0.42, colorWithAlpha(color, 0.12))
    gradient.addColorStop(1, colorWithAlpha(color, 0))
    context.shadowBlur = 0
    context.globalAlpha = Math.min(0.52, (0.14 + ratio * 0.28 + beatLift * 0.16) * intensity)
    context.fillStyle = gradient
    context.beginPath()
    context.arc(x, y, radius, 0, TAU)
    context.fill()
  }

  for (let index = 0; index < particleCount; index += 1) {
    const seedA = pseudoRandom(index * 11 + 3)
    const seedB = pseudoRandom(index * 17 + 9)
    const seedC = pseudoRandom(index * 23 + 5)
    const bandIndex = Math.floor(seedA * 96)
    const ratio = frequencyRatio(data, bandIndex, 96, time, motion)
    const shaped = Math.pow(ratio, 0.5)
    const bandPulse = clamp(shaped * 0.72 + reactionLift * 0.38, 0, 1.35)
    const angle = seedA * TAU + time * 0.00005 * driftScale * (seedB > 0.5 ? 1 : -1)
    const radius = maxRadius * (0.12 + seedB * 0.84) * (0.88 + shaped * 0.18 + bassLift * 0.14)
    const wobble =
      Math.sin(time * 0.001 * driftScale + index * 0.73) *
      maxRadius *
      (0.014 + props.energy.treble * 0.035)
    const x = centerX + Math.cos(angle) * radius + Math.cos(angle * 1.7) * wobble
    const y = centerY + Math.sin(angle) * radius * 0.68 + Math.sin(angle * 1.3) * wobble
    const size = (0.8 + seedC * 2 + shaped * 5.8 + beatLift * 2.8) * intensity * playFactor
    const color = seedA < 0.34 ? palette.low : seedA < 0.68 ? palette.mid : palette.high

    if (index % 9 === 0 || (bandPulse > 0.86 && index % 6 === 0)) {
      const haloRadius = Math.max(7, size * (3.2 + bandPulse * 2.8))

      context.globalAlpha = Math.min(0.5, (0.1 + bandPulse * 0.24 + beatLift * 0.12) * intensity)
      context.fillStyle = colorWithAlpha(color, 0.28)
      context.shadowColor = color
      context.shadowBlur = 12 + bandPulse * 22
      context.beginPath()
      context.arc(x, y, haloRadius * 0.42, 0, TAU)
      context.fill()
    }

    context.globalAlpha = Math.min(0.92, 0.1 + shaped * 0.72 + reactionLift * 0.2)
    context.fillStyle = color
    context.shadowColor = color
    context.shadowBlur = 7 + bandPulse * 22
    context.beginPath()
    context.arc(x, y, Math.max(0.8, size), 0, TAU)
    context.fill()

    context.globalAlpha = Math.min(0.96, 0.34 + shaped * 0.46 + beatLift * 0.18)
    context.fillStyle = palette.high
    context.shadowColor = palette.high
    context.shadowBlur = 5 + bandPulse * 12
    context.beginPath()
    context.arc(x, y, Math.max(0.45, size * 0.34), 0, TAU)
    context.fill()

    if (!props.reducedMotion && bandPulse > 0.7 && seedC > 0.58 && index % 11 === 0) {
      const twinkle = (0.45 + Math.sin(time * 0.012 * motion + index * 1.37) * 0.35 + bandPulse * 0.2) * intensity
      const arm = size * (1.8 + twinkle * 2.4)
      const rotation = seedB * TAU + time * 0.00028 * motion
      const diagonal = rotation + Math.PI / 2

      context.globalAlpha = Math.min(0.68, twinkle)
      context.strokeStyle = colorWithAlpha(palette.high, 0.86)
      context.lineWidth = Math.max(0.7, size * 0.14)
      context.shadowColor = palette.high
      context.shadowBlur = 8 + bandPulse * 12
      context.beginPath()
      context.moveTo(x - Math.cos(rotation) * arm, y - Math.sin(rotation) * arm * 0.7)
      context.lineTo(x + Math.cos(rotation) * arm, y + Math.sin(rotation) * arm * 0.7)
      context.moveTo(x - Math.cos(diagonal) * arm * 0.55, y - Math.sin(diagonal) * arm * 0.38)
      context.lineTo(x + Math.cos(diagonal) * arm * 0.55, y + Math.sin(diagonal) * arm * 0.38)
      context.stroke()
    }
  }

  if (reactionLift > 0.28) {
    const rayCount = props.reducedMotion ? 8 : 18
    const rotation = props.reducedMotion ? 0 : time * 0.00018 * motion

    context.lineCap = 'round'
    for (let index = 0; index < rayCount; index += 1) {
      const progress = index / rayCount
      const ratio = Math.pow(frequencyRatio(data, index * 5, rayCount * 5, time, motion), 0.58)
      const angle = progress * TAU + rotation
      const inner = maxRadius * (0.1 + bassLift * 0.04)
      const outer = maxRadius * (0.28 + ratio * 0.42 + beatLift * 0.18)
      const startX = centerX + Math.cos(angle) * inner
      const startY = centerY + Math.sin(angle) * inner * 0.68
      const endX = centerX + Math.cos(angle) * outer
      const endY = centerY + Math.sin(angle) * outer * 0.68
      const color = progress < 0.34 ? palette.low : progress < 0.68 ? palette.mid : palette.high

      context.globalAlpha = Math.min(0.46, (0.08 + ratio * 0.24 + beatLift * 0.18) * intensity)
      context.strokeStyle = color
      context.lineWidth = Math.max(1.2, 1.2 + ratio * 3.4 + bassLift * 1.2)
      context.shadowColor = color
      context.shadowBlur = 8 + ratio * 18 + beatLift * 18
      context.beginPath()
      context.moveTo(startX, startY)
      context.lineTo(endX, endY)
      context.stroke()
    }
  }

  const coreGradient = context.createRadialGradient(centerX, centerY, 0, centerX, centerY, maxRadius * 0.46)
  coreGradient.addColorStop(0, palette.pulse)
  coreGradient.addColorStop(0.32, palette.wave)
  coreGradient.addColorStop(1, 'transparent')
  context.shadowBlur = 0
  context.globalAlpha = Math.min(0.34, 0.08 + volumeLift * 0.14 + beatLift * 0.12 + bassLift * 0.08)
  context.fillStyle = coreGradient
  context.fillRect(0, 0, canvasWidth, canvasHeight)
  context.restore()
}

function drawRibbonSpectrum(
  context: CanvasRenderingContext2D,
  palette: VisualPalette,
  time: number,
  intensity: number,
  motion: number,
  playFactor: number,
) {
  const data = props.frequencyData
  const sampleCount = 112
  const layerColors = [palette.low, palette.mid, palette.high]
  const baseY = canvasHeight * (props.mode === 'sleep' ? 0.58 : 0.5)
  const layerGap = canvasHeight * 0.075
  const baseAmplitude = canvasHeight * (props.mode === 'focus' ? 0.052 : 0.082) * intensity * playFactor

  context.save()
  context.globalCompositeOperation = 'lighter'
  context.lineCap = 'round'
  context.lineJoin = 'round'

  for (let layer = 0; layer < layerColors.length; layer += 1) {
    const yOffset = (layer - 1) * layerGap
    const phase = time * (0.0011 + layer * 0.00026) * motion
    const amplitude = baseAmplitude * (1 + layer * 0.2)

    context.beginPath()
    for (let index = 0; index <= sampleCount; index += 1) {
      const progress = index / sampleCount
      const ratio = frequencyRatio(data, index + layer * 8, sampleCount + layer * 8, time, motion)
      const harmonic =
        Math.sin(progress * TAU * (2 + layer) + phase) * amplitude * 0.34 +
        Math.sin(progress * TAU * (5 + layer) - phase * 0.8) * amplitude * 0.18
      const y = baseY + yOffset + (ratio - 0.42) * amplitude * 1.45 + harmonic
      const x = progress * canvasWidth

      if (index === 0) {
        context.moveTo(x, y)
      } else {
        context.lineTo(x, y)
      }
    }

    context.globalAlpha = 0.28 + props.energy.volume * 0.28 + layer * 0.08
    context.strokeStyle = layerColors[layer]
    context.lineWidth = Math.max(2.2, canvasHeight * (0.004 + layer * 0.0012))
    context.shadowColor = layerColors[layer]
    context.shadowBlur = 10 + props.energy.treble * 18
    context.stroke()
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
  if (props.lineStyle === 'none') {
    return
  }

  if (props.lineStyle === 'beam') {
    drawBeamLines(context, palette, time, intensity, motion, playFactor)
    return
  }

  if (props.lineStyle === 'scan') {
    drawScanLines(context, palette, time, intensity, motion, playFactor)
    return
  }

  if (props.lineStyle === 'constellation') {
    drawConstellationLines(context, palette, time, intensity, motion, playFactor)
    return
  }

  drawWaveLines(context, palette, time, intensity, motion, playFactor)
}

function drawWaveLines(
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

function drawBeamLines(
  context: CanvasRenderingContext2D,
  palette: VisualPalette,
  time: number,
  intensity: number,
  motion: number,
  playFactor: number,
) {
  const beamCount = props.reducedMotion ? 3 : 5
  const centerY = canvasHeight * (props.mode === 'sleep' ? 0.5 : 0.44)
  const spread = canvasHeight * 0.14
  const phase = time * 0.0012 * motion

  context.save()
  context.globalCompositeOperation = 'lighter'
  context.lineCap = 'round'

  for (let index = 0; index < beamCount; index += 1) {
    const offset = (index - (beamCount - 1) / 2) * (spread / Math.max(1, beamCount - 1))
    const energy = index % 2 === 0 ? props.energy.mid : props.energy.treble
    const y = centerY + offset + Math.sin(phase + index * 0.82) * spread * 0.18 * motion
    const glow = (0.22 + energy * 0.5 + props.energy.beat * 0.18) * intensity * playFactor
    const gradient = context.createLinearGradient(0, y, canvasWidth, y)

    gradient.addColorStop(0, 'transparent')
    gradient.addColorStop(0.18, index % 2 === 0 ? palette.low : palette.wave)
    gradient.addColorStop(0.5, palette.high)
    gradient.addColorStop(0.82, index % 2 === 0 ? palette.mid : palette.pulse)
    gradient.addColorStop(1, 'transparent')
    context.globalAlpha = Math.min(0.72, glow)
    context.strokeStyle = gradient
    context.lineWidth = Math.max(2, canvasHeight * (0.004 + energy * 0.01))
    context.shadowColor = index % 2 === 0 ? palette.wave : palette.pulse
    context.shadowBlur = 14 + energy * 24
    context.beginPath()
    context.moveTo(canvasWidth * 0.04, y)
    context.bezierCurveTo(
      canvasWidth * 0.28,
      y - spread * 0.18,
      canvasWidth * 0.72,
      y + spread * 0.18,
      canvasWidth * 0.96,
      y,
    )
    context.stroke()
  }

  context.restore()
}

function drawScanLines(
  context: CanvasRenderingContext2D,
  palette: VisualPalette,
  time: number,
  intensity: number,
  motion: number,
  playFactor: number,
) {
  const lineCount = props.reducedMotion ? 6 : 10
  const gap = canvasHeight / (lineCount + 2)
  const drift = ((time * 0.045 * motion) % gap) - gap

  context.save()
  context.globalCompositeOperation = 'lighter'
  context.lineCap = 'round'

  for (let index = 0; index < lineCount; index += 1) {
    const y = gap * (index + 1) + drift
    const ratio = frequencyRatio(props.frequencyData, index * 6, lineCount * 6, time, motion)
    const alpha = (0.08 + ratio * 0.34 + props.energy.beat * 0.08) * intensity * playFactor
    const gradient = context.createLinearGradient(0, y, canvasWidth, y)

    gradient.addColorStop(0, 'transparent')
    gradient.addColorStop(0.16, palette.low)
    gradient.addColorStop(0.48, palette.mid)
    gradient.addColorStop(0.84, palette.high)
    gradient.addColorStop(1, 'transparent')
    context.globalAlpha = Math.min(0.58, alpha)
    context.strokeStyle = gradient
    context.lineWidth = Math.max(1, 1.2 + ratio * 3.2)
    context.shadowColor = palette.wave
    context.shadowBlur = ratio * 12
    context.beginPath()
    context.moveTo(canvasWidth * 0.08, y)
    context.lineTo(canvasWidth * 0.92, y + Math.sin(time * 0.001 * motion + index) * 10 * motion)
    context.stroke()
  }

  context.globalAlpha = 0.16 + props.energy.treble * 0.2 * playFactor
  context.strokeStyle = palette.pulse
  context.lineWidth = 1
  for (let index = 0; index < 5; index += 1) {
    const x = ((time * 0.035 * motion + index * canvasWidth * 0.24) % (canvasWidth * 1.2)) - canvasWidth * 0.1
    context.beginPath()
    context.moveTo(x, canvasHeight * 0.16)
    context.lineTo(x + canvasWidth * 0.05, canvasHeight * 0.86)
    context.stroke()
  }

  context.restore()
}

function drawConstellationLines(
  context: CanvasRenderingContext2D,
  palette: VisualPalette,
  time: number,
  intensity: number,
  motion: number,
  playFactor: number,
) {
  const pointCount = props.reducedMotion ? 15 : 22
  const centerX = canvasWidth * 0.5
  const centerY = canvasHeight * 0.48
  const beatLift = props.energy.beat * playFactor
  const bassLift = props.energy.bass * playFactor
  const volumeLift = props.energy.volume * playFactor
  const reactionLift = clamp(beatLift * 0.62 + bassLift * 0.46 + volumeLift * 0.24, 0, 1.35)
  const points: Array<{ x: number; y: number; energy: number; color: string }> = []

  for (let index = 0; index < pointCount; index += 1) {
    const seedA = pseudoRandom(index * 29 + 7)
    const seedB = pseudoRandom(index * 31 + 11)
    const ratio = frequencyRatio(props.frequencyData, index * 4, pointCount * 4, time, motion)
    const energy = clamp(Math.pow(ratio, 0.58) + reactionLift * 0.24, 0, 1.32)
    const baseX = canvasWidth * (0.1 + seedA * 0.8)
    const baseY = canvasHeight * (0.18 + seedB * 0.58)
    const centerDistanceX = baseX - centerX
    const centerDistanceY = baseY - centerY
    const distance = Math.max(1, Math.hypot(centerDistanceX, centerDistanceY))
    const burst = reactionLift * energy * (props.reducedMotion ? 0.45 : 1)
    const drift = Math.sin(time * 0.0007 * motion + index * 0.9) * 18 * motion * (0.55 + energy)

    points.push({
      x: baseX + drift + (centerDistanceX / distance) * burst * canvasWidth * 0.038,
      y: baseY - drift * 0.42 + (centerDistanceY / distance) * burst * canvasHeight * 0.032,
      energy,
      color: index % 3 === 0 ? palette.low : index % 3 === 1 ? palette.mid : palette.high,
    })
  }

  context.save()
  context.globalCompositeOperation = 'lighter'
  context.lineCap = 'round'

  for (let index = 0; index < pointCount - 1; index += 1) {
    const point = points[index]
    const next = points[(index + 2) % pointCount]
    const strength = clamp((point.energy + next.energy) / 2 + reactionLift * 0.18, 0, 1.5)

    context.globalAlpha = Math.min(0.68, (0.1 + strength * 0.48 + beatLift * 0.16) * intensity * playFactor)
    context.strokeStyle = point.color
    context.lineWidth = Math.max(0.9, 0.9 + strength * 3.8 + bassLift * 1.1)
    context.shadowColor = next.color
    context.shadowBlur = 6 + strength * 20 + beatLift * 12
    context.beginPath()
    context.moveTo(point.x, point.y)
    context.lineTo(next.x, next.y)
    context.stroke()

    if (strength > 0.62 && index % 3 === 0) {
      const far = points[(index + 5) % pointCount]
      context.globalAlpha = Math.min(0.32, (strength - 0.42) * 0.36 + beatLift * 0.08)
      context.lineWidth = Math.max(0.8, 0.8 + strength * 1.8)
      context.shadowBlur = 8 + strength * 14
      context.beginPath()
      context.moveTo(point.x, point.y)
      context.lineTo(far.x, far.y)
      context.stroke()
    }
  }

  for (const point of points) {
    const size = (1.4 + point.energy * 6.4 + beatLift * 2.2) * intensity * playFactor
    context.globalAlpha = Math.min(0.94, 0.18 + point.energy * 0.64 + beatLift * 0.14)
    context.fillStyle = point.color
    context.shadowColor = point.color
    context.shadowBlur = 8 + point.energy * 24 + beatLift * 14
    context.beginPath()
    context.arc(point.x, point.y, Math.max(0.8, size), 0, TAU)
    context.fill()
  }

  if (reactionLift > 0.18) {
    context.globalAlpha = Math.min(0.28, reactionLift * 0.24)
    context.strokeStyle = palette.pulse
    context.lineWidth = Math.max(1, 1 + bassLift * 2.2)
    context.shadowColor = palette.pulse
    context.shadowBlur = 14 + reactionLift * 24
    context.beginPath()
    context.ellipse(
      centerX,
      centerY,
      canvasWidth * (0.18 + reactionLift * 0.08),
      canvasHeight * (0.11 + reactionLift * 0.05),
      0,
      0,
      TAU,
    )
    context.stroke()
  }

  context.restore()
}

function drawPulse(
  context: CanvasRenderingContext2D,
  palette: VisualPalette,
  pulse: number,
  time: number,
  motion: number,
) {
  if (props.rippleStyle === 'none') {
    return
  }

  if (props.rippleStyle === 'water') {
    drawWaterRipple(context, palette, pulse, time, motion)
    return
  }

  if (props.rippleStyle === 'heartbeat') {
    drawHeartbeatRipple(context, palette, pulse, time, motion)
    return
  }

  if (props.rippleStyle === 'halo') {
    drawHaloRipple(context, palette, pulse, time, motion)
    return
  }

  drawRingRipple(context, palette, pulse, time, motion)
}

function drawRingRipple(
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

function drawWaterRipple(
  context: CanvasRenderingContext2D,
  palette: VisualPalette,
  pulse: number,
  time: number,
  motion: number,
) {
  const layerCount = props.reducedMotion ? 3 : 5
  const centerY = canvasHeight * (props.mode === 'sleep' ? 0.62 : 0.66)
  const amplitude = canvasHeight * (0.018 + props.energy.bass * 0.045 + pulse * 0.05)
  const phase = time * 0.0015 * motion

  context.save()
  context.globalCompositeOperation = 'lighter'
  context.lineCap = 'round'

  for (let layer = 0; layer < layerCount; layer += 1) {
    const yBase = centerY + (layer - (layerCount - 1) / 2) * canvasHeight * 0.045
    context.beginPath()
    for (let x = 0; x <= canvasWidth; x += Math.max(8, canvasWidth / 120)) {
      const progress = x / canvasWidth
      const y =
        yBase +
        Math.sin(progress * TAU * (2.2 + layer * 0.4) + phase + layer) * amplitude +
        Math.sin(progress * TAU * (5.4 + layer * 0.2) - phase * 0.7) * amplitude * 0.36
      if (x === 0) {
        context.moveTo(x, y)
      } else {
        context.lineTo(x, y)
      }
    }

    context.globalAlpha = Math.min(0.42, 0.1 + pulse * 0.28 + layer * 0.035)
    context.strokeStyle = layer % 2 === 0 ? palette.wave : palette.pulse
    context.lineWidth = Math.max(1.2, canvasHeight * 0.0028)
    context.shadowColor = palette.wave
    context.shadowBlur = 8 + pulse * 18
    context.stroke()
  }

  context.restore()
}

function drawHeartbeatRipple(
  context: CanvasRenderingContext2D,
  palette: VisualPalette,
  pulse: number,
  time: number,
  motion: number,
) {
  const centerX = canvasWidth * 0.5
  const centerY = canvasHeight * 0.5
  const beat = Math.max(pulse, props.energy.bass * 0.52 + props.energy.beat * 0.48)
  const baseRadius = Math.min(canvasWidth, canvasHeight) * (0.13 + beat * 0.08)
  const squeeze = 0.46 + props.energy.mid * 0.18

  context.save()
  context.globalCompositeOperation = 'lighter'
  context.lineCap = 'round'
  context.lineWidth = Math.max(2, canvasHeight * (0.004 + beat * 0.006))
  context.shadowColor = palette.pulse
  context.shadowBlur = 16 + beat * 32

  for (let index = 0; index < 3; index += 1) {
    const offset = props.reducedMotion ? index * 0.16 : ((time * 0.00035 * motion + index * 0.24) % 0.72)
    const radius = baseRadius + offset * Math.min(canvasWidth, canvasHeight) * 0.34
    const alpha = Math.max(0, 0.46 - offset * 0.58) + beat * 0.18
    context.globalAlpha = Math.min(0.72, alpha)
    context.strokeStyle = index % 2 === 0 ? palette.pulse : palette.high
    context.beginPath()
    context.ellipse(centerX, centerY, radius * 1.68, radius * squeeze, 0, 0, TAU)
    context.stroke()
  }

  context.globalAlpha = Math.min(0.5, 0.12 + beat * 0.42)
  context.strokeStyle = palette.high
  context.beginPath()
  context.moveTo(centerX - baseRadius * 1.4, centerY)
  context.lineTo(centerX - baseRadius * 0.52, centerY)
  context.lineTo(centerX - baseRadius * 0.34, centerY - baseRadius * 0.34 * beat)
  context.lineTo(centerX - baseRadius * 0.08, centerY + baseRadius * 0.3 * beat)
  context.lineTo(centerX + baseRadius * 0.18, centerY - baseRadius * 0.42 * beat)
  context.lineTo(centerX + baseRadius * 0.42, centerY)
  context.lineTo(centerX + baseRadius * 1.4, centerY)
  context.stroke()

  context.restore()
}

function drawHaloRipple(
  context: CanvasRenderingContext2D,
  palette: VisualPalette,
  pulse: number,
  time: number,
  motion: number,
) {
  const centerX = canvasWidth * 0.5
  const centerY = canvasHeight * 0.5
  const maxRadius = Math.min(canvasWidth, canvasHeight) * (0.34 + pulse * 0.16)
  const breathing = props.reducedMotion ? 0.5 : 0.5 + Math.sin(time * 0.0012 * motion) * 0.12
  const gradient = context.createRadialGradient(centerX, centerY, 0, centerX, centerY, maxRadius)

  gradient.addColorStop(0, palette.high)
  gradient.addColorStop(0.18, palette.pulse)
  gradient.addColorStop(0.52, palette.wave)
  gradient.addColorStop(1, 'transparent')

  context.save()
  context.globalCompositeOperation = 'lighter'
  context.globalAlpha = Math.min(0.34, 0.08 + pulse * 0.2 + props.energy.volume * 0.12) * breathing
  context.fillStyle = gradient
  context.fillRect(0, 0, canvasWidth, canvasHeight)

  context.globalAlpha = Math.min(0.32, 0.08 + pulse * 0.2)
  context.strokeStyle = palette.high
  context.lineWidth = Math.max(1.4, canvasHeight * 0.003)
  context.beginPath()
  context.ellipse(centerX, centerY, maxRadius * 1.22, maxRadius * 0.58, 0, 0, TAU)
  context.stroke()
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

function pseudoRandom(seed: number) {
  const value = Math.sin(seed * 12.9898) * 43758.5453
  return value - Math.floor(value)
}

function colorWithAlpha(color: string, alpha: number) {
  const safeAlpha = clamp(alpha, 0, 1)
  const hex = color.trim()
  const shortHexMatch = /^#([0-9a-f]{3})$/i.exec(hex)
  const hexMatch = /^#([0-9a-f]{6})$/i.exec(hex)

  if (shortHexMatch) {
    const [, value] = shortHexMatch
    const red = parseInt(value[0] + value[0], 16)
    const green = parseInt(value[1] + value[1], 16)
    const blue = parseInt(value[2] + value[2], 16)
    return `rgba(${red}, ${green}, ${blue}, ${safeAlpha})`
  }

  if (hexMatch) {
    const [, value] = hexMatch
    const red = parseInt(value.slice(0, 2), 16)
    const green = parseInt(value.slice(2, 4), 16)
    const blue = parseInt(value.slice(4, 6), 16)
    return `rgba(${red}, ${green}, ${blue}, ${safeAlpha})`
  }

  return color
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

function visualPalette(theme: MusicImmersiveTheme | string, mode: MusicVisualMode): VisualPalette {
  if (theme === 'animal-island') {
    return {
      backgroundStart: mode === 'sleep' ? '#2c3140' : '#244f54',
      backgroundMiddle: mode === 'focus' ? '#5a7f71' : '#2d756b',
      backgroundEnd: '#f4bd77',
      low: '#ffd97b',
      mid: '#7fb6a6',
      high: '#fff8e9',
      wave: '#ffe7ad',
      pulse: '#bfe8d0',
    }
  }

  if (theme === 'cinema') {
    return {
      backgroundStart: '#010304',
      backgroundMiddle: mode === 'dance' ? '#2a0f17' : '#0b1116',
      backgroundEnd: '#1f1a12',
      low: '#ff5367',
      mid: '#7ad7c2',
      high: '#fff1bd',
      wave: '#f4d28a',
      pulse: '#fff4d6',
    }
  }

  if (theme === 'galaxy') {
    return {
      backgroundStart: '#050608',
      backgroundMiddle: mode === 'sleep' ? '#111827' : '#102a44',
      backgroundEnd: '#3b2f73',
      low: '#73a7ff',
      mid: '#9cffdf',
      high: '#fff0b8',
      wave: '#8fe9ff',
      pulse: '#9cffdf',
    }
  }

  if (theme === 'neon') {
    return {
      backgroundStart: '#07071d',
      backgroundMiddle: mode === 'focus' ? '#0d2a42' : '#20104f',
      backgroundEnd: '#012f46',
      low: '#ff4fd8',
      mid: '#00f5d4',
      high: '#eff7ff',
      wave: '#74f7ff',
      pulse: '#ff4fd8',
    }
  }

  if (theme === 'sunset') {
    return {
      backgroundStart: '#1e0d16',
      backgroundMiddle: mode === 'sleep' ? '#332033' : '#6b2d26',
      backgroundEnd: '#d78a45',
      low: '#ff8a5c',
      mid: '#f4d28a',
      high: '#fff8df',
      wave: '#ffd28a',
      pulse: '#ff705f',
    }
  }

  if (theme === 'midnight') {
    return {
      backgroundStart: '#050812',
      backgroundMiddle: '#111827',
      backgroundEnd: mode === 'dance' ? '#27365e' : '#18243a',
      low: '#6f93c7',
      mid: '#9fb7d9',
      high: '#eef6ff',
      wave: '#9fb7d9',
      pulse: '#b8cdfa',
    }
  }

  if (mode === 'sleep') {
    return {
      backgroundStart: '#111827',
      backgroundMiddle: '#263754',
      backgroundEnd: '#4b5e7b',
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
