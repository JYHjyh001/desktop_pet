<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { DEFAULT_MUSIC_STAGE_TUNING } from '../types/app'
import type {
  BoxGeometry,
  BufferGeometry,
  Color,
  Group,
  InstancedBufferAttribute,
  InstancedMesh,
  IUniform,
  Object3D,
  PerspectiveCamera,
  Points,
  Scene,
  ShaderMaterial,
  Texture,
  WebGLRenderer,
} from 'three'
import type { MusicEnergyFrame } from '../composables/useMusicAudioAnalyzer'
import type {
  MusicImmersiveTheme,
  MusicLineStyle,
  MusicRippleStyle,
  MusicStageTuning,
  MusicSpectrumStyle,
  MusicVisualStagePreset,
} from '../types/app'

type MusicVisualMode = 'rhythm' | 'dance' | 'focus' | 'sleep'
type MusicStageTuningKey = keyof MusicStageTuning
type ThreeModule = typeof import('three')
type VisualKind = 'dust' | 'nebula' | 'stage' | 'burst' | 'trail'
type VisualKindCounts = Record<VisualKind, number>

const DJ_TERRAIN_CENTER_Z = -3.28
const TERRAIN_FLUX_HISTORY_SIZE = 40
const TERRAIN_RIPPLE_LIMIT = 12

interface TerrainCell {
  x: number
  z: number
  normalizedX: number
  normalizedZ: number
  band: number
  lane: number
  distance: number
  edgeFade: number
  ring: number
  angle: number
  spiral: number
  seed: number
  size: number
}

type TerrainRippleKind = 'pulse' | 'snare' | 'spark'

interface TerrainRipple {
  x: number
  z: number
  startedAt: number
  strength: number
  speedScale: number
  kind: TerrainRippleKind
  seed: number
}

interface ActiveTerrainRipple extends TerrainRipple {
  radius: number
  width: number
  fade: number
}

interface TerrainRippleInfluence {
  lift: number
  glow: number
  sharp: number
}

interface TerrainFluxTriggerState {
  kind: TerrainRippleKind
  bandStartRatio: number
  bandEndRatio: number
  sensitivity: number
  cooldownSeconds: number
  strengthScale: number
  history: number[]
  historyIndex: number
  smoothedFlux: number
  prevSmoothedFlux: number
  cooldownRemaining: number
  lastEnergy: number
  lastThreshold: number
}

interface VisualPalette {
  low: string
  mid: string
  high: string
  pulse: string
  shadow: string
  line: string
  ripple: string
  halo: string
}

interface StarUniforms {
  [key: string]: IUniform
  uTime: { value: number }
  uRhythmTime: { value: number }
  uBass: { value: number }
  uMid: { value: number }
  uTreble: { value: number }
  uBeat: { value: number }
  uVolume: { value: number }
  uIntensity: { value: number }
  uPlaying: { value: number }
  uReducedMotion: { value: number }
  uPixelRatio: { value: number }
  uLayerScale: { value: number }
  uLayerAlpha: { value: number }
  uStagePreset: { value: number }
  uStageHeight: { value: number }
  uStageResponse: { value: number }
  uStageWave: { value: number }
  uStageTrigger: { value: number }
  uStageLayerHeight: { value: number }
  uSpectrumStyle: { value: number }
  uLineStyle: { value: number }
  uRippleStyle: { value: number }
  uDotTex: { value: Texture }
  uColorLow: { value: Color }
  uColorMid: { value: Color }
  uColorHigh: { value: Color }
  uColorPulse: { value: Color }
  uColorShadow: { value: Color }
  uColorLine: { value: Color }
  uColorRipple: { value: Color }
  uColorHalo: { value: Color }
}

interface TerrainUniforms {
  [key: string]: IUniform
  uOpacity: { value: number }
  uFogColor: { value: Color }
}

interface SmoothedMusicEnergy {
  bass: number
  mid: number
  treble: number
  beat: number
  volume: number
  rhythm: number
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

const props = withDefaults(
  defineProps<{
    energy: MusicEnergyFrame
    frequencyData: Uint8Array | null
    playing: boolean
    mode: MusicVisualMode
    stagePreset: MusicVisualStagePreset
    spectrumStyle: MusicSpectrumStyle
    lineStyle: MusicLineStyle
    rippleStyle: MusicRippleStyle
    intensity: number
    reducedMotion: boolean
    stageTuning: MusicStageTuning
    theme: MusicImmersiveTheme | string
    stageYaw: number
    stagePitch: number
    stageDragging: boolean
    freeCamera: ImmersiveFreeCameraView
  }>(),
  {
    mode: 'rhythm',
    frequencyData: null,
    stagePreset: 'galaxy',
    spectrumStyle: 'particles',
    lineStyle: 'wave',
    rippleStyle: 'rings',
    intensity: 0.72,
    reducedMotion: false,
    stageTuning: () => ({ ...DEFAULT_MUSIC_STAGE_TUNING }),
    theme: 'light',
    stageYaw: 0,
    stagePitch: 0,
    stageDragging: false,
    freeCamera: () => ({
      active: false,
      locked: false,
      x: 0,
      y: 0.7,
      z: 8.2,
      yaw: 0,
      pitch: -0.08,
      roll: 0,
      fov: 46,
    }),
  },
)

const emit = defineEmits<{
  (event: 'webgl-unavailable', reason: string): void
}>()

const canvas = ref<HTMLCanvasElement | null>(null)

let renderer: WebGLRenderer | null = null
let scene: Scene | null = null
let camera: PerspectiveCamera | null = null
let sceneGroup: Group | null = null
let terrainMesh: InstancedMesh | null = null
let terrainGeometry: BoxGeometry | null = null
let terrainMaterial: ShaderMaterial | null = null
let terrainUniforms: TerrainUniforms | null = null
let terrainColorAttribute: InstancedBufferAttribute | null = null
let terrainFogAttribute: InstancedBufferAttribute | null = null
let terrainDummy: Object3D | null = null
let terrainCells: TerrainCell[] = []
let terrainHeights = new Float32Array(0)
let terrainRipples: TerrainRipple[] = []
let terrainRippleIndex = 0
let terrainRippleSeed = 0
let terrainFluxTriggers = createTerrainFluxTriggers()
let previousTerrainFrequencyData: Float32Array | null = null
let lastTerrainTriggerUpdateTime = 0
let lastTerrainBeatLevel = 0
let lastTerrainEnergyLevel = 0
let lastTerrainBassRippleTime = -999
let lastTerrainMidRippleTime = -999
let lastTerrainSparkRippleTime = -999
let terrainScratchColor: Color | null = null
let terrainLowColor: Color | null = null
let terrainMidColor: Color | null = null
let terrainHighColor: Color | null = null
let terrainPulseColor: Color | null = null
let terrainShadowColor: Color | null = null
let geometry: BufferGeometry | null = null
let mainMaterial: ShaderMaterial | null = null
let glowMaterial: ShaderMaterial | null = null
let mainUniforms: StarUniforms | null = null
let glowUniforms: StarUniforms | null = null
let mainPoints: Points | null = null
let glowPoints: Points | null = null
let dotTexture: Texture | null = null
let resizeObserver: ResizeObserver | null = null
let animationFrameId: number | null = null
let contextLost = false
let threeRuntime: ThreeModule | null = null
let disposed = false
let currentAspect = 1
let orbitYaw = 0
let orbitPitch = 0.08
let orbitRadius = 8.2
let cameraPunch = 0
let lastRenderSeconds: number | null = null
let rhythmTime = 0
let rhythmVelocity = 0.7
let smoothedEnergy = createSilentSmoothedEnergy()

onMounted(() => {
  disposed = false
  void initializeWebgl()
})

onBeforeUnmount(() => {
  disposed = true
  disposeWebgl()
})

watch(
  () => [props.reducedMotion, props.stagePreset] as const,
  () => {
    rebuildGeometry()
    resizeRenderer()
  },
)

watch(
  () => terrainDensitySignature(),
  () => {
    rebuildTerrainLayer()
  },
)

watch(
  () => starDensitySignature(),
  () => {
    rebuildGeometry()
  },
)

async function initializeWebgl() {
  const element = canvas.value
  if (!element) {
    return
  }

  try {
    const three = await import('three')
    if (disposed || element !== canvas.value) {
      return
    }

    contextLost = false
    threeRuntime = three
    renderer = new three.WebGLRenderer({
      canvas: element,
      alpha: true,
      antialias: false,
      powerPreference: props.reducedMotion ? 'low-power' : 'high-performance',
    })
    renderer.setClearColor(0x000000, 0)
    renderer.outputColorSpace = three.SRGBColorSpace

    scene = new three.Scene()
    camera = new three.PerspectiveCamera(46, 1, 0.1, 80)
    camera.position.set(0, 0.7, orbitRadius)

    sceneGroup = new three.Group()
    scene.add(sceneGroup)

    createTerrainLayer(three)

    dotTexture = createDotTexture(three)
    geometry = createStarGeometry(three)
    mainUniforms = createUniforms(three, dotTexture, 1, 0.92)
    glowUniforms = createUniforms(three, dotTexture, 4.8, 0.22)
    mainMaterial = createMaterial(three, mainUniforms)
    glowMaterial = createMaterial(three, glowUniforms)

    glowPoints = new three.Points(geometry, glowMaterial)
    glowPoints.frustumCulled = false
    glowPoints.renderOrder = 0
    sceneGroup.add(glowPoints)

    mainPoints = new three.Points(geometry, mainMaterial)
    mainPoints.frustumCulled = false
    mainPoints.renderOrder = 1
    sceneGroup.add(mainPoints)

    element.addEventListener('webglcontextlost', handleContextLost)
    resizeObserver = new ResizeObserver(() => resizeRenderer())
    resizeObserver.observe(element)
    resizeRenderer()
    render(0)
  } catch (err) {
    emit('webgl-unavailable', `WebGL 3D 沉浸场景初始化失败：${String(err)}`)
    disposeWebgl()
  }
}

function createMaterial(three: ThreeModule, uniforms: StarUniforms) {
  return new three.ShaderMaterial({
    uniforms,
    vertexShader: vertexShaderSource,
    fragmentShader: fragmentShaderSource,
    transparent: true,
    depthWrite: false,
    depthTest: true,
    blending: three.AdditiveBlending,
  })
}

function createTerrainLayer(three: ThreeModule) {
  if (!sceneGroup || props.stagePreset !== 'dj') {
    terrainCells = []
    terrainHeights = new Float32Array(0)
    return
  }

  terrainCells = createTerrainCells(props.stagePreset, props.reducedMotion, stageTuningValue('density', 0.35, 1.7))
  terrainHeights = new Float32Array(terrainCells.length)
  resetTerrainRipples()
  terrainGeometry = new three.BoxGeometry(1, 1, 1)
  terrainColorAttribute = new three.InstancedBufferAttribute(new Float32Array(terrainCells.length * 3), 3)
  terrainColorAttribute.setUsage(three.DynamicDrawUsage)
  terrainFogAttribute = new three.InstancedBufferAttribute(new Float32Array(terrainCells.length), 1)
  for (let index = 0; index < terrainCells.length; index += 1) {
    terrainFogAttribute.setX(index, terrainCells[index].edgeFade)
  }
  terrainGeometry.setAttribute('aTerrainColor', terrainColorAttribute)
  terrainGeometry.setAttribute('aTerrainFog', terrainFogAttribute)
  terrainUniforms = {
    uOpacity: { value: terrainLayerOpacity(props.stagePreset) },
    uFogColor: { value: new three.Color('#050812') },
  }
  terrainMaterial = new three.ShaderMaterial({
    uniforms: terrainUniforms,
    vertexShader: terrainVertexShaderSource,
    fragmentShader: terrainFragmentShaderSource,
    transparent: false,
    depthTest: true,
    depthWrite: true,
    blending: three.NoBlending,
    side: three.DoubleSide,
    toneMapped: false,
  })
  terrainMesh = new three.InstancedMesh(terrainGeometry, terrainMaterial, terrainCells.length)
  terrainMesh.frustumCulled = false
  terrainMesh.renderOrder = 2
  terrainMesh.instanceMatrix.setUsage(three.DynamicDrawUsage)
  terrainDummy = new three.Object3D()
  terrainScratchColor = new three.Color()
  terrainLowColor = new three.Color()
  terrainMidColor = new three.Color()
  terrainHighColor = new three.Color()
  terrainPulseColor = new three.Color()
  terrainShadowColor = new three.Color()
  sceneGroup.add(terrainMesh)
  updateTerrainLayer(0, true)
}

function rebuildTerrainLayer() {
  const three = threeRuntime
  if (!three || !sceneGroup) {
    return
  }

  if (terrainMesh) {
    sceneGroup.remove(terrainMesh)
  }
  terrainGeometry?.dispose()
  terrainMaterial?.dispose()
  terrainMesh = null
  terrainGeometry = null
  terrainMaterial = null
  terrainUniforms = null
  terrainColorAttribute = null
  terrainFogAttribute = null
  terrainDummy = null
  terrainCells = []
  terrainHeights = new Float32Array(0)
  resetTerrainRipples()
  createTerrainLayer(three)
}

function createTerrainCells(
  preset: MusicVisualStagePreset,
  reducedMotion: boolean,
  densityTuning = 1,
): TerrainCell[] {
  const gridSize = terrainGridSize(preset, reducedMotion, densityTuning)
  const djGridExtent = 1.2
  const djOuterLimit = 1.08
  const footprint = preset === 'galaxy' ? 7.4 : preset === 'lyric' ? 6.4 : preset === 'dj' ? 12.2 : 8.8
  const effectiveFootprint = preset === 'dj' ? footprint * djGridExtent : footprint
  const spacing = effectiveFootprint / (preset === 'dj' ? gridSize : Math.max(1, gridSize - 1))
  const baseSize = spacing * (preset === 'dj' ? 1.015 : 0.72)
  const cells: TerrainCell[] = []

  for (let row = 0; row < gridSize; row += 1) {
    const rowN = preset === 'dj' ? (row + 0.5) / gridSize : row / Math.max(1, gridSize - 1)
    for (let column = 0; column < gridSize; column += 1) {
      const columnN = preset === 'dj' ? (column + 0.5) / gridSize : column / Math.max(1, gridSize - 1)
      const normalizedX = preset === 'dj' ? (columnN - 0.5) * 2 * djGridExtent : (columnN - 0.5) * 2
      const normalizedZ = preset === 'dj' ? (rowN - 0.5) * 2 * djGridExtent : (rowN - 0.5) * 2
      const x = preset === 'dj' ? normalizedX * footprint * 0.5 : (columnN - 0.5) * footprint
      const z = preset === 'dj' ? normalizedZ * footprint * 0.5 : (rowN - 0.5) * footprint - 1.35
      const radialDistance = Math.hypot(normalizedX, normalizedZ)
      if (preset === 'dj' && radialDistance > djOuterLimit) {
        continue
      }
      const distance = preset === 'dj' ? clamp01(radialDistance / djOuterLimit) : clamp01(radialDistance / 1.42)
      const ring = clamp01(distance)
      const rimFade = preset === 'dj' ? smoothstep(0.94, djOuterLimit, radialDistance) : 0
      const edgeFade = preset === 'dj' ? rimFade * 0.62 : 0
      const angle = Math.atan2(normalizedZ, normalizedX)
      const ridge = (Math.sin(angle * 3 + columnN * Math.PI * 2) + 1) * 0.5
      const seed = pseudoRandom((row + 1) * 37.13 + (column + 1) * 19.91)
      const djAngularFlow =
        Math.sin(angle * 2 + ring * 5.6) * 0.46 +
        Math.cos(angle * 3 - ring * 3.8) * 0.34 +
        Math.sin(Math.sin(angle) * 2.6 + Math.cos(angle) * 1.7 + ring * 4.2) * 0.2
      const djRingTexture = (Math.sin(angle * 4 + ring * 2.7) + 1) * 0.5
      const spiral = preset === 'dj' ? clamp01(0.5 + djAngularFlow * 0.5) : ridge
      let band = clamp01(distance * 0.68 + ridge * 0.2 + columnN * 0.12)

      if (preset === 'dj') {
        band = clamp01(distance * 0.8 + spiral * 0.12 + djRingTexture * 0.06 + seed * 0.02)
      } else if (preset === 'cinematic') {
        band = clamp01(distance * 0.5 + rowN * 0.34 + ridge * 0.16)
      } else if (preset === 'lyric') {
        band = clamp01(0.28 + columnN * 0.44)
      }

      cells.push({
        x,
        z,
        normalizedX,
        normalizedZ,
        band,
        lane: rowN,
        distance,
        edgeFade,
        ring,
        angle,
        spiral,
        seed,
        size: preset === 'dj' ? baseSize : baseSize * (0.88 + seed * 0.18),
      })
    }
  }

  return cells
}

function updateTerrainLayer(time: number, immediate = false) {
  if (!terrainMesh || !terrainUniforms || !terrainColorAttribute || !terrainDummy || !terrainScratchColor) {
    return
  }

  const playFactor = props.playing ? 1 : 0.28
  const bass = clamp01(props.energy.bass * playFactor)
  const mid = clamp01(props.energy.mid * playFactor)
  const treble = clamp01(props.energy.treble * playFactor)
  const beat = clamp01(props.energy.beat * playFactor)
  const volume = clamp01(props.energy.volume * playFactor)
  const intensity = clamp(props.intensity, 0.2, 1)
  const preset = props.stagePreset
  if (preset !== 'dj') {
    return
  }
  const heightTuning = stageTuningValue('height', 0.25, 2.4)
  const responseTuning = stageTuningValue('response', 0.2, 2.4)
  const waveTuning = stageTuningValue('wave', 0.2, 2.4)
  const triggerTuning = stageTuningValue('trigger', 0.25, 2.4)
  const maxHeight = terrainMaxHeight(preset) * (0.58 + intensity * 0.24) * heightTuning
  const baseHeight = 0.048
  const responseBase = props.reducedMotion ? 0.075 : 0.14
  const response = immediate ? 1 : clamp(responseBase * responseTuning, 0.025, 0.36)
  const groundY = -1.86
  const palette = visualPalette(props.theme, props.mode)
  const fogColor = terrainFogColor(props.theme, props.mode)
  const lowMid = clamp01(bass * 0.28 + mid * 0.72)
  const highMid = clamp01(mid * 0.42 + treble * 0.58)
  const subBass = clamp01(bass * 0.56 + beat * 0.2)
  const energy = clamp01(volume * 0.46 + bass * 0.22 + mid * 0.18 + treble * 0.14)
  const triggerEnergy = clamp01(subBass * 0.34 + bass * 0.32 + beat * 0.22 + volume * 0.12)

  updateTerrainRippleTriggers(time, {
    frequencyData: props.frequencyData,
    playing: props.playing,
    subBass,
    bass,
    lowMid,
    mid,
    treble,
    beat,
    volume,
    energy,
    triggerTuning,
    reducedMotion: props.reducedMotion,
  })
  const activeRipples = activeTerrainRipples(time, props.reducedMotion, waveTuning)

  terrainUniforms.uOpacity.value = terrainLayerOpacity(preset)
  terrainUniforms.uFogColor.value.set(fogColor)
  terrainLowColor?.set(palette.low)
  terrainMidColor?.set(palette.mid)
  terrainHighColor?.set(palette.high)
  terrainPulseColor?.set(palette.pulse)
  terrainShadowColor?.set(fogColor)

  for (let index = 0; index < terrainCells.length; index += 1) {
    const cell = terrainCells[index]
    const visibleFade = 1 - cell.edgeFade * 0.34
    const innerFade = 0.5 + smoothstep(0.02, 0.2, cell.ring) * 0.5
    const radialGate = innerFade * visibleFade
    const centerCore = 1 - smoothstep(0.02, 0.2, cell.ring)
    const innerBand = 1 - smoothstep(0.12, 0.44, cell.ring)
    const middleBand = 1 - smoothstep(0, 0.26, Math.abs(cell.ring - (0.42 + bass * 0.025)))
    const outerBand = smoothstep(0.48, 0.86, cell.ring) * (1 - cell.edgeFade * 0.28)
    const ripple = terrainRippleInfluence(cell, activeRipples)
    const localWave = clamp01(ripple.lift * waveTuning)
    const rippleGlow = clamp01(ripple.glow * (0.72 + waveTuning * 0.28))
    const rippleTail = (Math.sin(cell.ring * 32 + cell.spiral * 2.4 - time * 0.34 + rippleGlow * 2.1) + 1) * 0.5
    const angularFlow =
      Math.sin(cell.angle * 2 + cell.ring * 3.1 + time * 0.18) * 0.58 +
      Math.cos(cell.angle * 3 - cell.ring * 2.4 - time * 0.14) * 0.42
    const radialPhase =
      cell.ring * 18 +
      Math.sin(cell.angle) * 1.45 +
      Math.cos(cell.angle) * 0.9 -
      time * 0.52 +
      cell.seed * 1.4
    const slowFlow =
      (Math.sin(cell.ring * 17.5 - time * 0.32 + cell.spiral * 2.2) +
        angularFlow) *
        0.25 +
      0.5
    const radialCurrent = Math.max(
      0,
      Math.sin(radialPhase),
    )
    const spikeGate = cell.seed > 0.925 ? Math.max(0, Math.sin(time * 5.8 + cell.seed * Math.PI * 2)) : 0
    const microSpark = cell.seed > 0.982 ? Math.max(0, Math.sin(time * 8.4 + cell.seed * Math.PI * 4)) : 0
    const coreLift = centerCore * (subBass * 0.34 + bass * 0.16 + beat * 0.1 + volume * 0.055)
    const bassChunkLift = bass * (0.1 + innerBand * 0.24 + middleBand * 0.14) * (0.58 + slowFlow * 0.42)
    const waveLift = localWave * (0.58 + rippleTail * 0.2) * (1 + innerBand * 0.26 + middleBand * 0.16)
    const lowMidLift = lowMid * slowFlow * (0.13 + middleBand * 0.21)
    const midLift = mid * radialCurrent * (0.12 + middleBand * 0.22 + outerBand * 0.08)
    const highMidLift = highMid * (spikeGate * outerBand * 0.12 + ripple.sharp * 0.22)
    const energySpike = microSpark * beat * energy * (0.07 + outerBand * 0.08)
    const terrainEnergy = clamp(
      (coreLift + bassChunkLift + lowMidLift + midLift + highMidLift + energySpike) * radialGate - 0.03,
      0,
      1.25,
    )
    const rippleEnergy = clamp(waveLift * radialGate, 0, 1.18)
    const rippleHeight = Math.pow(rippleEnergy, 0.82) * maxHeight * (0.24 + waveTuning * 0.08)
    const ringRipple = (Math.sin(cell.ring * 34 - time * 0.32 + cell.spiral * 3.1) * 0.005 + (slowFlow - 0.5) * 0.007) * radialGate
    const horizonBase = baseHeight * (0.3 + radialGate * 0.7)
    const targetHeight =
      horizonBase +
      Math.pow(terrainEnergy, 0.86) * maxHeight +
      rippleHeight +
      ringRipple

    const height = clamp(targetHeight, 0.006, maxHeight * 1.18 + 0.1)
    terrainHeights[index] += (height - terrainHeights[index]) * response
    const currentHeight = Math.max(0.006, terrainHeights[index])
    const heightRatio = clamp01(currentHeight / Math.max(0.1, maxHeight))

    const currentSize = cell.size * (0.68 + radialGate * 0.32)

    terrainDummy.position.set(cell.x, groundY + currentHeight * 0.5, cell.z)
    terrainDummy.scale.set(currentSize, currentHeight, currentSize)
    terrainDummy.updateMatrix()
    terrainMesh.setMatrixAt(index, terrainDummy.matrix)

    if (terrainLowColor && terrainMidColor && terrainHighColor && terrainPulseColor && terrainShadowColor) {
      const waveGlow = clamp01(rippleGlow * (0.72 + triggerEnergy * 0.24) + localWave * 0.1 + centerCore * subBass * 0.1 + spikeGate * highMid * 0.12)
      terrainScratchColor.copy(terrainLowColor)
      terrainScratchColor.lerp(terrainMidColor, clamp01(cell.ring * 0.42 + slowFlow * 0.18 + lowMid * 0.08))
      terrainScratchColor.lerp(terrainHighColor, clamp01(waveGlow * 0.22 + heightRatio * 0.08 + highMidLift * 0.26))
      terrainScratchColor.lerp(
        terrainPulseColor,
        clamp01(waveGlow * beat * 0.08 + localWave * (0.08 + triggerEnergy * 0.08) + ripple.sharp * 0.1 + energySpike * 0.18),
      )
      terrainScratchColor.lerp(terrainShadowColor, clamp01(cell.edgeFade * 0.38))
      terrainScratchColor.multiplyScalar(0.36 + radialGate * 0.28 + heightRatio * 0.18 + waveGlow * 0.2 + energy * 0.035)
      terrainScratchColor.r = clamp(terrainScratchColor.r, 0, 0.72)
      terrainScratchColor.g = clamp(terrainScratchColor.g, 0, 0.72)
      terrainScratchColor.b = clamp(terrainScratchColor.b, 0, 0.72)
      terrainColorAttribute.setXYZ(index, terrainScratchColor.r, terrainScratchColor.g, terrainScratchColor.b)
    }
  }

  terrainMesh.instanceMatrix.needsUpdate = true
  terrainColorAttribute.needsUpdate = true
}

function terrainGridSize(preset: MusicVisualStagePreset, reducedMotion: boolean, densityTuning = 1) {
  if (preset === 'dj') {
    const baseSize = reducedMotion ? 120 : 176
    const minSize = reducedMotion ? 54 : 82
    const maxSize = reducedMotion ? 220 : 300
    return Math.round(clamp(baseSize * densityTuning, minSize, maxSize))
  }
  if (reducedMotion) {
    return 34
  }
  if (preset === 'cinematic') {
    return 58
  }
  if (preset === 'galaxy') {
    return 42
  }
  if (preset === 'lyric') {
    return 38
  }
  return 52
}

function terrainMaxHeight(preset: MusicVisualStagePreset) {
  if (preset === 'cinematic') {
    return 3.6
  }
  if (preset === 'dj') {
    return 1.34
  }
  if (preset === 'galaxy') {
    return 0.95
  }
  if (preset === 'lyric') {
    return 0.72
  }
  return 2.55
}

function terrainLayerOpacity(preset: MusicVisualStagePreset) {
  if (preset === 'galaxy') {
    return 0
  }
  if (preset === 'lyric') {
    return 0.24
  }
  if (preset === 'cinematic') {
    return 0.86
  }
  if (preset === 'dj') {
    return 1
  }
  return 0.76
}

function disposeWebgl() {
  if (animationFrameId !== null) {
    window.cancelAnimationFrame(animationFrameId)
    animationFrameId = null
  }

  canvas.value?.removeEventListener('webglcontextlost', handleContextLost)
  resizeObserver?.disconnect()
  resizeObserver = null

  if (mainPoints && sceneGroup) {
    sceneGroup.remove(mainPoints)
  }
  if (glowPoints && sceneGroup) {
    sceneGroup.remove(glowPoints)
  }
  if (terrainMesh && sceneGroup) {
    sceneGroup.remove(terrainMesh)
  }
  if (sceneGroup && scene) {
    scene.remove(sceneGroup)
  }

  geometry?.dispose()
  terrainGeometry?.dispose()
  terrainMaterial?.dispose()
  mainMaterial?.dispose()
  glowMaterial?.dispose()
  dotTexture?.dispose()
  renderer?.dispose()

  renderer = null
  scene = null
  camera = null
  sceneGroup = null
  terrainMesh = null
  terrainGeometry = null
  terrainMaterial = null
  terrainUniforms = null
  terrainColorAttribute = null
  terrainFogAttribute = null
  terrainDummy = null
  terrainCells = []
  terrainHeights = new Float32Array(0)
  resetTerrainRipples()
  terrainScratchColor = null
  terrainLowColor = null
  terrainMidColor = null
  terrainHighColor = null
  terrainPulseColor = null
  terrainShadowColor = null
  geometry = null
  mainMaterial = null
  glowMaterial = null
  mainUniforms = null
  glowUniforms = null
  mainPoints = null
  glowPoints = null
  dotTexture = null
  threeRuntime = null
  lastRenderSeconds = null
  rhythmTime = 0
  rhythmVelocity = 0.7
  smoothedEnergy = createSilentSmoothedEnergy()
}

function handleContextLost(event: Event) {
  event.preventDefault()
  if (contextLost) {
    return
  }

  contextLost = true
  emit('webgl-unavailable', 'WebGL 上下文已丢失，已回退 Canvas 可视化。')
}

function resizeRenderer() {
  const element = canvas.value
  if (!element || !renderer || !camera) {
    return
  }

  const rect = element.getBoundingClientRect()
  const width = Math.max(1, Math.floor(rect.width))
  const height = Math.max(1, Math.floor(rect.height))
  const cssPixels = Math.max(1, width * height)
  const targetBudget = props.reducedMotion ? 1800000 : 3600000
  const budgetRatio = Math.sqrt(targetBudget / cssPixels)
  const pixelRatio = Math.min(window.devicePixelRatio || 1, props.reducedMotion ? 1 : 1.35, budgetRatio)

  renderer.setPixelRatio(Math.max(0.68, pixelRatio))
  renderer.setSize(width, height, false)

  currentAspect = width / Math.max(1, height)
  camera.aspect = currentAspect
  camera.updateProjectionMatrix()
}

function render(time: number) {
  animationFrameId = window.requestAnimationFrame(render)
  if (!renderer || !scene || !camera || !sceneGroup || !mainUniforms || !glowUniforms || contextLost) {
    return
  }

  const seconds = time * 0.001
  const deltaSeconds = lastRenderSeconds === null ? 1 / 60 : clamp(seconds - lastRenderSeconds, 0, 0.08)
  lastRenderSeconds = seconds
  updateRhythmClock(deltaSeconds)
  updateCamera(seconds)
  updateSceneGroup(seconds)
  updateTerrainLayer(seconds)
  updateUniforms(mainUniforms, seconds)
  updateUniforms(glowUniforms, seconds)
  renderer.render(scene, camera)
}

function updateCamera(time: number) {
  if (!camera) {
    return
  }

  if (props.freeCamera.active || props.freeCamera.locked) {
    updateFreeCamera(time)
    return
  }

  const motionScale = props.reducedMotion ? 0.38 : 1
  const energy = props.energy
  const isDjPreset = props.stagePreset === 'dj'
  const cameraTuning =
    props.stagePreset === 'galaxy' ? stageTuningValue('camera', 0.55, 1.75) : stageTuningValue('camera', 0.55, 1.65)
  const cinematicBoost = props.stagePreset === 'cinematic' ? 1.42 : isDjPreset ? 0.58 : 1
  const driftScale = props.stagePreset === 'galaxy' ? 1.36 : props.stagePreset === 'lyric' ? 0.38 : isDjPreset ? 0.16 : 1
  const radiusBias = props.stagePreset === 'galaxy' ? 0.62 : props.stagePreset === 'lyric' ? 1.18 : isDjPreset ? 2.05 : 0
  const dragYaw = degreesToRadians(props.stageYaw) * 1.18 * motionScale
  const dragPitch = degreesToRadians(props.stagePitch) * 0.96 * motionScale
  const idleYaw = Math.sin(time * 0.075) * 0.055 * motionScale * driftScale
  const idlePitch = Math.cos(time * 0.062) * 0.035 * motionScale * driftScale
  const beatPunch = props.playing ? clamp01(energy.beat * 0.85 + energy.bass * 0.35) : 0
  const targetYaw = dragYaw + idleYaw + (isDjPreset ? -0.18 : 0)
  const targetPitch = isDjPreset
    ? clamp(dragPitch * 0.42 + idlePitch + 0.26, 0.16, 0.38)
    : clamp(dragPitch + idlePitch + 0.08, -0.38, 0.42)
  const targetRadius = isDjPreset
    ? (8.15 + radiusBias) * cameraTuning - beatPunch * 0.08 * motionScale - clamp(props.intensity, 0.2, 1) * 0.08
    : (8.15 + radiusBias) * cameraTuning -
      beatPunch * 0.42 * motionScale * cinematicBoost -
      clamp(props.intensity, 0.2, 1) * 0.22

  orbitYaw += (targetYaw - orbitYaw) * (props.stageDragging ? 0.22 : 0.105)
  orbitPitch += (targetPitch - orbitPitch) * 0.1
  orbitRadius += (targetRadius - orbitRadius) * 0.075
  cameraPunch = Math.max(cameraPunch * 0.86, beatPunch * 0.72 * motionScale * cinematicBoost)

  const cy = Math.cos(orbitPitch)
  const sy = Math.sin(orbitPitch)
  const st = Math.sin(orbitYaw)
  const ct = Math.cos(orbitYaw)
  if (isDjPreset) {
    const targetY = -0.92
    camera.position.set(orbitRadius * cy * st, targetY + orbitRadius * sy, DJ_TERRAIN_CENTER_Z + orbitRadius * cy * ct)
    camera.lookAt(0, targetY, DJ_TERRAIN_CENTER_Z)
  } else {
    camera.position.set(orbitRadius * cy * st, 0.7 + orbitRadius * sy, orbitRadius * cy * ct)
    camera.lookAt(0, 0, -0.45)
  }
  camera.rotation.z += Math.sin(time * 0.9) * cameraPunch * 0.008 * cinematicBoost
  const targetFov = isDjPreset ? 46 - cameraPunch * 0.62 : 46 - cameraPunch * 2.6 * cinematicBoost
  camera.fov += (targetFov - camera.fov) * 0.12
  camera.updateProjectionMatrix()
}

function updateFreeCamera(_time: number) {
  if (!camera) {
    return
  }

  const motionScale = props.reducedMotion ? 0.42 : 1
  const energy = props.energy
  const beatPunch = props.playing ? clamp01(energy.beat * 0.72 + energy.bass * 0.28) : 0
  const cameraShake = beatPunch * 0.028 * motionScale
  const targetFov = clamp(props.freeCamera.fov - beatPunch * 1.4 * motionScale, 26, 74)

  camera.position.set(props.freeCamera.x, props.freeCamera.y, props.freeCamera.z)
  camera.rotation.order = 'YXZ'
  camera.rotation.set(
    props.freeCamera.pitch + Math.sin(energy.mid * 6.2) * cameraShake,
    props.freeCamera.yaw + Math.sin(energy.treble * 5.4) * cameraShake * 0.7,
    props.freeCamera.roll + Math.sin(energy.bass * 4.8) * cameraShake * 0.35,
  )
  camera.fov += (targetFov - camera.fov) * (targetFov < camera.fov ? 0.24 : 0.12)
  camera.updateProjectionMatrix()
}

function updateSceneGroup(time: number) {
  if (!sceneGroup) {
    return
  }

  const energy = props.energy
  const motionScale = props.reducedMotion ? 0.36 : 1
  const presetSpin =
    props.stagePreset === 'galaxy' ? 1.72 : props.stagePreset === 'dj' ? 0.18 : props.stagePreset === 'lyric' ? 0.42 : 1
  const presetLift =
    props.stagePreset === 'cinematic' ? 1.36 : props.stagePreset === 'dj' ? 0.26 : props.stagePreset === 'lyric' ? 0.44 : 1
  const beatLift = props.playing ? clamp01(energy.beat * 0.75 + energy.bass * 0.35) * presetLift : 0
  const freeCameraMode = props.freeCamera.active || props.freeCamera.locked
  sceneGroup.rotation.y =
    time * 0.018 * motionScale * presetSpin +
    (freeCameraMode ? 0 : degreesToRadians(props.stageYaw) * 0.16 * motionScale)
  sceneGroup.rotation.x = freeCameraMode ? 0 : -degreesToRadians(props.stagePitch) * 0.08 * motionScale
  sceneGroup.position.y = props.stagePreset === 'dj' ? -0.16 : 0
  sceneGroup.position.z =
    props.stagePreset === 'dj' ? DJ_TERRAIN_CENTER_Z - beatLift * 0.02 * motionScale : -0.45 - beatLift * 0.16 * motionScale
  const scale = props.stagePreset === 'dj' ? 1.08 + beatLift * 0.003 * motionScale : 1 + beatLift * 0.018 * motionScale
  sceneGroup.scale.setScalar(scale)
}

function updateUniforms(uniforms: StarUniforms, time: number) {
  const palette = visualPalette(props.theme, props.mode)
  const pixelRatio = Math.min(window.devicePixelRatio || 1, props.reducedMotion ? 1 : 1.35)

  uniforms.uTime.value = time
  uniforms.uRhythmTime.value = rhythmTime
  uniforms.uBass.value = smoothedEnergy.bass
  uniforms.uMid.value = smoothedEnergy.mid
  uniforms.uTreble.value = smoothedEnergy.treble
  uniforms.uBeat.value = smoothedEnergy.beat
  uniforms.uVolume.value = smoothedEnergy.volume
  uniforms.uIntensity.value = clamp(props.intensity, 0.2, 1)
  uniforms.uPlaying.value = props.playing ? 1 : 0
  uniforms.uReducedMotion.value = props.reducedMotion ? 1 : 0
  uniforms.uPixelRatio.value = pixelRatio
  uniforms.uStagePreset.value = stagePresetCode(props.stagePreset)
  uniforms.uStageHeight.value =
    props.stagePreset === 'galaxy' ? stageTuningValue('height', 0.45, 2.2) : stageTuningValue('height', 0.25, 2.4)
  uniforms.uStageResponse.value =
    props.stagePreset === 'galaxy' ? stageTuningValue('response', 0.25, 2.4) : stageTuningValue('response', 0.2, 2.4)
  uniforms.uStageWave.value =
    props.stagePreset === 'galaxy' ? stageTuningValue('wave', 0.15, 2.6) : stageTuningValue('wave', 0.2, 2.4)
  uniforms.uStageTrigger.value =
    props.stagePreset === 'galaxy' ? stageTuningValue('trigger', 0.1, 2.8) : stageTuningValue('trigger', 0.25, 2.4)
  uniforms.uStageLayerHeight.value = props.stagePreset === 'galaxy' ? stageTuningValue('layerHeight', 0.25, 2.8) : 1
  uniforms.uSpectrumStyle.value = spectrumStyleCode(props.spectrumStyle)
  uniforms.uLineStyle.value = lineStyleCode(props.lineStyle)
  uniforms.uRippleStyle.value = rippleStyleCode(props.rippleStyle)
  uniforms.uColorLow.value.set(palette.low)
  uniforms.uColorMid.value.set(palette.mid)
  uniforms.uColorHigh.value.set(palette.high)
  uniforms.uColorPulse.value.set(palette.pulse)
  uniforms.uColorShadow.value.set(palette.shadow)
  uniforms.uColorLine.value.set(palette.line)
  uniforms.uColorRipple.value.set(palette.ripple)
  uniforms.uColorHalo.value.set(palette.halo)
}

function updateRhythmClock(deltaSeconds: number) {
  const playFactor = props.playing ? 1 : 0
  const targetBass = clamp01(props.energy.bass * playFactor)
  const targetMid = clamp01(props.energy.mid * playFactor)
  const targetTreble = clamp01(props.energy.treble * playFactor)
  const targetBeat = clamp01(props.energy.beat * playFactor)
  const targetVolume = clamp01(props.energy.volume * playFactor)
  const attack = 1 - Math.exp(-deltaSeconds * 11.5)
  const release = 1 - Math.exp(-deltaSeconds * 4.2)

  smoothedEnergy.bass = smoothAudioValue(smoothedEnergy.bass, targetBass, attack, release)
  smoothedEnergy.mid = smoothAudioValue(smoothedEnergy.mid, targetMid, attack, release)
  smoothedEnergy.treble = smoothAudioValue(smoothedEnergy.treble, targetTreble, attack, release)
  smoothedEnergy.beat = smoothAudioValue(smoothedEnergy.beat, targetBeat, attack * 1.18, release * 0.88)
  smoothedEnergy.volume = smoothAudioValue(smoothedEnergy.volume, targetVolume, attack * 0.82, release)

  const rhythmTarget = clamp01(
    smoothedEnergy.beat * 0.44 +
      smoothedEnergy.bass * 0.3 +
      smoothedEnergy.volume * 0.18 +
      smoothedEnergy.mid * 0.06 +
      smoothedEnergy.treble * 0.02,
  )
  smoothedEnergy.rhythm = smoothAudioValue(smoothedEnergy.rhythm, rhythmTarget, attack * 0.82, release * 0.72)

  const responseTuning =
    props.stagePreset === 'galaxy' ? stageTuningValue('response', 0.25, 2.4) : stageTuningValue('response', 0.2, 2.4)
  const triggerTuning =
    props.stagePreset === 'galaxy' ? stageTuningValue('trigger', 0.1, 2.8) : stageTuningValue('trigger', 0.25, 2.4)
  const baseSpeed = props.playing ? 0.72 : 0.18
  const rhythmBoost = smoothedEnergy.rhythm * (0.92 + responseTuning * 0.5)
  const beatBoost = smoothedEnergy.beat * (0.28 + triggerTuning * 0.2)
  const bassBoost = smoothedEnergy.bass * 0.34
  const reducedMotionScale = props.reducedMotion ? 0.42 : 1
  const targetVelocity = (baseSpeed + rhythmBoost + beatBoost + bassBoost) * reducedMotionScale
  const velocityEase = targetVelocity > rhythmVelocity ? attack * 0.78 : release * 0.58

  rhythmVelocity += (targetVelocity - rhythmVelocity) * clamp(velocityEase, 0.02, 0.34)
  rhythmTime += deltaSeconds * rhythmVelocity
}

function smoothAudioValue(current: number, target: number, attack: number, release: number) {
  const factor = target > current ? attack : release
  return current + (target - current) * clamp(factor, 0.02, 0.55)
}

function rebuildGeometry() {
  const three = threeRuntime
  if (!mainPoints || !glowPoints || !three) {
    return
  }

  const previous = geometry
  geometry = createStarGeometry(three)
  mainPoints.geometry = geometry
  glowPoints.geometry = geometry
  previous?.dispose()
  rebuildTerrainLayer()
}

function createStarGeometry(three: ThreeModule) {
  const counts = visualKindCounts(
    props.stagePreset,
    props.reducedMotion,
    props.stagePreset === 'galaxy' ? stageTuningValue('density', 0.35, 2) : 1,
  )
  const total = counts.dust + counts.nebula + counts.stage + counts.burst + counts.trail
  const positions = new Float32Array(total * 3)
  const seeds = new Float32Array(total)
  const kinds = new Float32Array(total)
  const lanes = new Float32Array(total)
  const bands = new Float32Array(total)
  const sizes = new Float32Array(total)
  let cursor = 0

  cursor = fillKind('dust', counts.dust, cursor, positions, seeds, kinds, lanes, bands, sizes)
  cursor = fillKind('nebula', counts.nebula, cursor, positions, seeds, kinds, lanes, bands, sizes)
  cursor = fillKind('stage', counts.stage, cursor, positions, seeds, kinds, lanes, bands, sizes)
  cursor = fillKind('burst', counts.burst, cursor, positions, seeds, kinds, lanes, bands, sizes)
  fillKind('trail', counts.trail, cursor, positions, seeds, kinds, lanes, bands, sizes)

  const nextGeometry = new three.BufferGeometry()
  nextGeometry.setAttribute('position', new three.BufferAttribute(positions, 3))
  nextGeometry.setAttribute('aSeed', new three.BufferAttribute(seeds, 1))
  nextGeometry.setAttribute('aKind', new three.BufferAttribute(kinds, 1))
  nextGeometry.setAttribute('aLane', new three.BufferAttribute(lanes, 1))
  nextGeometry.setAttribute('aBand', new three.BufferAttribute(bands, 1))
  nextGeometry.setAttribute('aSize', new three.BufferAttribute(sizes, 1))
  return nextGeometry
}

function visualKindCounts(
  preset: MusicVisualStagePreset,
  reducedMotion: boolean,
  densityTuning = 1,
): VisualKindCounts {
  if (preset === 'dj') {
    return { dust: 0, nebula: 0, stage: 0, burst: 0, trail: 0 }
  }

  if (preset === 'galaxy') {
    const counts = reducedMotion
      ? { dust: 760, nebula: 260, stage: 240, burst: 90, trail: 140 }
      : { dust: 4200, nebula: 1680, stage: 320, burst: 140, trail: 460 }
    return scaleVisualKindCounts(counts, densityTuning)
  }

  if (reducedMotion) {
    return { dust: 520, nebula: 180, stage: 360, burst: 110, trail: 120 }
  }

  if (preset === 'cinematic') {
    return { dust: 260, nebula: 80, stage: 520, burst: 360, trail: 160 }
  }

  if (preset === 'lyric') {
    return { dust: 160, nebula: 40, stage: 120, burst: 24, trail: 60 }
  }

  return { dust: 420, nebula: 120, stage: 360, burst: 180, trail: 140 }
}

function scaleVisualKindCounts(counts: VisualKindCounts, densityTuning: number): VisualKindCounts {
  const density = clamp(densityTuning, 0.35, 2)
  return {
    dust: Math.round(counts.dust * density),
    nebula: Math.round(counts.nebula * density),
    stage: Math.round(counts.stage * (0.72 + density * 0.28)),
    burst: Math.round(counts.burst * (0.78 + density * 0.22)),
    trail: Math.round(counts.trail * (0.66 + density * 0.34)),
  }
}

function fillKind(
  kind: VisualKind,
  count: number,
  cursor: number,
  positions: Float32Array,
  seeds: Float32Array,
  kinds: Float32Array,
  lanes: Float32Array,
  bands: Float32Array,
  sizes: Float32Array,
) {
  const kindCode = { dust: 0, nebula: 1, stage: 2, burst: 3, trail: 4 }[kind]

  for (let offset = 0; offset < count; offset += 1) {
    const index = cursor + offset
    const lane = count <= 1 ? 0 : offset / (count - 1)
    const seed = pseudoRandom(index * 17.31 + 4.7)
    const seedX = pseudoRandom(index * 23.7 + 11)
    const seedY = pseudoRandom(index * 29.1 + 19)
    const seedZ = pseudoRandom(index * 31.9 + 3)
    const band = pseudoRandom(index * 37.3 + 5.4)
    const base = index * 3

    if (kind === 'dust') {
      const radius = 2.4 + seedX * 7.6
      const theta = seedY * Math.PI * 2
      const phi = (seedZ - 0.5) * 1.48
      positions[base] = Math.cos(theta) * Math.cos(phi) * radius
      positions[base + 1] = Math.sin(phi) * radius * 0.82
      positions[base + 2] = Math.sin(theta) * Math.cos(phi) * radius - 2.1
      sizes[index] = 1.1 + seed * 3.6
    } else if (kind === 'nebula') {
      const arm = Math.floor(lane * 5)
      const armLane = lane * 5 - arm
      const angle = arm * Math.PI * 0.4 + armLane * 2.25 + (seedX - 0.5) * 0.48
      const radius = 1.0 + armLane * 3.8 + seedY * 0.9
      positions[base] = Math.cos(angle) * radius
      positions[base + 1] = (seedZ - 0.5) * 2.2 + Math.sin(armLane * Math.PI) * 0.32
      positions[base + 2] = Math.sin(angle) * radius - 1.45
      sizes[index] = 2.2 + seed * 5.2
    } else if (kind === 'stage') {
      if (props.stagePreset === 'galaxy') {
        const cluster = Math.floor(seedX * 7)
        const clusterSeed = pseudoRandom(cluster * 41.7 + 8.2)
        const clusterAngle = clusterSeed * Math.PI * 2
        const clusterRadius = 1.25 + pseudoRandom(cluster * 23.5 + 4.6) * 4.35
        const localAngle = seedY * Math.PI * 2
        const localRadius = Math.pow(seedZ, 0.58) * (0.34 + clusterSeed * 0.86)
        positions[base] =
          Math.cos(clusterAngle) * clusterRadius +
          Math.cos(localAngle) * localRadius +
          (seed - 0.5) * 0.42
        positions[base + 1] =
          (pseudoRandom(cluster * 17.2 + 2.3) - 0.5) * 2.3 +
          Math.sin(localAngle) * localRadius * 0.58 +
          (seedY - 0.5) * 0.46
        positions[base + 2] =
          Math.sin(clusterAngle) * clusterRadius -
          2.0 +
          Math.sin(localAngle) * localRadius +
          (seedX - 0.5) * 0.52
        sizes[index] = 1.25 + seed * 3.4
        bands[index] = seedX
        lanes[index] = seedY
        seeds[index] = seed
        kinds[index] = kindCode
        continue
      }

      const columns = 84
      const column = offset % columns
      const stack = Math.floor(offset / columns)
      const columnN = column / Math.max(1, columns - 1)
      const stackN = stack / Math.max(1, Math.ceil(count / columns) - 1)
      const arc = (columnN - 0.5) * Math.PI * 1.52
      const radius = 2.0 + stackN * 1.42
      positions[base] = Math.sin(arc) * radius
      positions[base + 1] = (stackN - 0.44) * 1.7
      positions[base + 2] = Math.cos(arc) * radius - 2.65
      sizes[index] = 1.9 + seed * 3.2
      bands[index] = columnN
      lanes[index] = stackN
      seeds[index] = seed
      kinds[index] = kindCode
      continue
    } else if (kind === 'burst') {
      if (props.stagePreset === 'galaxy') {
        const cloud = Math.floor(seedY * 9)
        const cloudSeed = pseudoRandom(cloud * 29.3 + 6.7)
        const cloudAngle = cloudSeed * Math.PI * 2
        const cloudRadius = 0.82 + pseudoRandom(cloud * 13.1 + 1.9) * 5.15
        const sprayAngle = seedX * Math.PI * 2
        const sprayRadius = Math.pow(seedZ, 0.44) * (0.2 + cloudSeed * 1.16)
        positions[base] =
          Math.cos(cloudAngle) * cloudRadius +
          Math.cos(sprayAngle) * sprayRadius +
          (seed - 0.5) * 0.28
        positions[base + 1] =
          (pseudoRandom(cloud * 19.7 + 3.4) - 0.5) * 1.8 +
          Math.sin(sprayAngle) * sprayRadius * 0.46
        positions[base + 2] =
          Math.sin(cloudAngle) * cloudRadius -
          1.95 +
          Math.sin(sprayAngle) * sprayRadius +
          (seedY - 0.5) * 0.36
        sizes[index] = 1.45 + seed * 3.8
        bands[index] = seedZ
        lanes[index] = seedX
        seeds[index] = seed
        kinds[index] = kindCode
        continue
      }

      const rings = 6
      const ring = offset % rings
      const segment = Math.floor(offset / rings)
      const segmentN = segment / Math.max(1, Math.ceil(count / rings) - 1)
      const angle = segmentN * Math.PI * 2
      const radius = 0.7 + ring * 0.64
      positions[base] = Math.cos(angle) * radius
      positions[base + 1] = Math.sin(angle) * radius * 0.36 + (seedZ - 0.5) * 0.12
      positions[base + 2] = -1.2 - ring * 0.32
      sizes[index] = 2.0 + seed * 3.4
      bands[index] = ring / Math.max(1, rings - 1)
      lanes[index] = segmentN
      seeds[index] = seed
      kinds[index] = kindCode
      continue
    } else {
      const angle = lane * Math.PI * 10 + seed * 0.7
      const radius = 1.8 + seedY * 4.1
      positions[base] = Math.cos(angle) * radius
      positions[base + 1] = (seedZ - 0.5) * 3.0
      positions[base + 2] = Math.sin(angle) * radius - 1.85
      sizes[index] = 1.6 + seed * 2.8
    }

    seeds[index] = seed
    kinds[index] = kindCode
    lanes[index] = lane
    bands[index] = band
  }

  return cursor + count
}

function createUniforms(
  three: ThreeModule,
  dot: Texture,
  layerScale: number,
  layerAlpha: number,
): StarUniforms {
  return {
    uTime: { value: 0 },
    uRhythmTime: { value: 0 },
    uBass: { value: 0 },
    uMid: { value: 0 },
    uTreble: { value: 0 },
    uBeat: { value: 0 },
    uVolume: { value: 0 },
    uIntensity: { value: clamp(props.intensity, 0.2, 1) },
    uPlaying: { value: props.playing ? 1 : 0 },
    uReducedMotion: { value: props.reducedMotion ? 1 : 0 },
    uPixelRatio: { value: 1 },
    uLayerScale: { value: layerScale },
    uLayerAlpha: { value: layerAlpha },
    uStagePreset: { value: stagePresetCode(props.stagePreset) },
    uStageHeight: { value: 1 },
    uStageResponse: { value: 1 },
    uStageWave: { value: 1 },
    uStageTrigger: { value: 1 },
    uStageLayerHeight: { value: 1 },
    uSpectrumStyle: { value: spectrumStyleCode(props.spectrumStyle) },
    uLineStyle: { value: lineStyleCode(props.lineStyle) },
    uRippleStyle: { value: rippleStyleCode(props.rippleStyle) },
    uDotTex: { value: dot },
    uColorLow: { value: new three.Color('#73a7ff') },
    uColorMid: { value: new three.Color('#9cffdf') },
    uColorHigh: { value: new three.Color('#fff0b8') },
    uColorPulse: { value: new three.Color('#9cffdf') },
    uColorShadow: { value: new three.Color('#2b4a82') },
    uColorLine: { value: new three.Color('#8fe9ff') },
    uColorRipple: { value: new three.Color('#73a7ff') },
    uColorHalo: { value: new three.Color('#fff0b8') },
  }
}

function createDotTexture(three: ThreeModule) {
  const textureCanvas = document.createElement('canvas')
  textureCanvas.width = 72
  textureCanvas.height = 72
  const context = textureCanvas.getContext('2d')

  if (context) {
    const gradient = context.createRadialGradient(36, 36, 0, 36, 36, 35)
    gradient.addColorStop(0, 'rgba(255,255,255,1)')
    gradient.addColorStop(0.24, 'rgba(255,255,255,0.88)')
    gradient.addColorStop(0.56, 'rgba(255,255,255,0.28)')
    gradient.addColorStop(1, 'rgba(255,255,255,0)')
    context.fillStyle = gradient
    context.fillRect(0, 0, 72, 72)
  }

  const texture = new three.CanvasTexture(textureCanvas)
  texture.colorSpace = three.SRGBColorSpace
  texture.minFilter = three.LinearFilter
  texture.magFilter = three.LinearFilter
  return texture
}

const terrainVertexShaderSource = `
attribute vec3 aTerrainColor;
attribute float aTerrainFog;

varying vec3 vTerrainColor;
varying vec3 vTerrainLocalPosition;
varying float vTerrainTop;
varying float vTerrainDepth;
varying float vTerrainFog;

void main() {
  vTerrainColor = aTerrainColor;
  vTerrainLocalPosition = position;
  vTerrainTop = position.y + 0.5;
  vTerrainFog = aTerrainFog;

  vec4 mvPosition = modelViewMatrix * instanceMatrix * vec4(position, 1.0);
  float depthFog = smoothstep(7.6, 15.8, -mvPosition.z);
  vTerrainFog = clamp(vTerrainFog * 0.4 + depthFog * 0.6, 0.0, 1.0);
  vTerrainDepth = clamp(1.0 - depthFog * 0.36, 0.5, 1.0);
  gl_Position = projectionMatrix * mvPosition;
}
`

const terrainFragmentShaderSource = `
precision highp float;

uniform float uOpacity;
uniform vec3 uFogColor;

varying vec3 vTerrainColor;
varying vec3 vTerrainLocalPosition;
varying float vTerrainTop;
varying float vTerrainDepth;
varying float vTerrainFog;

void main() {
  float topGlow = smoothstep(0.68, 1.0, vTerrainTop);
  float sideGlow = max(
    smoothstep(0.36, 0.5, abs(vTerrainLocalPosition.x)),
    smoothstep(0.36, 0.5, abs(vTerrainLocalPosition.z))
  );
  float baseLight = 0.5 + topGlow * 0.22 + sideGlow * 0.1;
  vec3 color = vTerrainColor * baseLight;
  color = mix(color, vec3(0.72, 0.74, 0.7), topGlow * 0.035);
  color = min(color, vec3(0.72));
  float farSilhouette = smoothstep(0.48, 0.92, vTerrainFog);
  vec3 distantColumn = mix(uFogColor * 0.7, uFogColor * 0.42, topGlow * 0.28 + sideGlow * 0.18);
  color = mix(color, distantColumn, farSilhouette * 0.42);
  color = mix(color, uFogColor * 0.6, vTerrainFog * 0.34);

  gl_FragColor = vec4(color * vTerrainDepth, 1.0);
}
`

const vertexShaderSource = `
precision highp float;

uniform float uTime;
uniform float uRhythmTime;
uniform float uBass;
uniform float uMid;
uniform float uTreble;
uniform float uBeat;
uniform float uVolume;
uniform float uIntensity;
uniform float uPlaying;
uniform float uReducedMotion;
uniform float uPixelRatio;
uniform float uLayerScale;
uniform float uLayerAlpha;
uniform float uStagePreset;
uniform float uStageHeight;
uniform float uStageResponse;
uniform float uStageWave;
uniform float uStageTrigger;
uniform float uStageLayerHeight;
uniform float uSpectrumStyle;
uniform float uLineStyle;
uniform float uRippleStyle;
uniform vec3 uColorLow;
uniform vec3 uColorMid;
uniform vec3 uColorHigh;
uniform vec3 uColorPulse;
uniform vec3 uColorShadow;
uniform vec3 uColorLine;
uniform vec3 uColorRipple;
uniform vec3 uColorHalo;

attribute float aSeed;
attribute float aKind;
attribute float aLane;
attribute float aBand;
attribute float aSize;

varying vec3 vColor;
varying float vAlpha;
varying float vGlow;

#define PI 3.14159265359
#define TAU 6.28318530718

mat2 rotate2d(float angle) {
  float s = sin(angle);
  float c = cos(angle);
  return mat2(c, -s, s, c);
}

float bandEnergy(float band) {
  if (band < 0.28) return uBass;
  if (band < 0.68) return uMid;
  return uTreble;
}

float presetMatch(float target) {
  return 1.0 - step(0.5, abs(uStagePreset - target));
}

void main() {
  float motion = mix(1.0, 0.34, uReducedMotion);
  float playMix = 0.34 + uPlaying * 0.66;
  float defaultPreset = presetMatch(0.0);
  float galaxyPreset = presetMatch(1.0);
  float cinematicPreset = presetMatch(2.0);
  float djPreset = presetMatch(3.0);
  float lyricPreset = presetMatch(4.0);
  float rawTime = uTime * motion;
  float rhythmTime = uRhythmTime * motion;
  float t = mix(rawTime, rhythmTime, galaxyPreset);
  float energy = bandEnergy(aBand);
  float twinkle = pow(0.5 + 0.5 * sin(t * (0.65 + aSeed * 1.25) + aSeed * TAU), 4.0);
  float stagePower = 1.0 + cinematicPreset * 0.62 + djPreset * 0.46 - lyricPreset * 0.54;
  float driftPower = 1.0 + galaxyPreset * 0.72 + djPreset * 0.18 - lyricPreset * 0.58;
  float galaxyDepth = mix(1.0, clamp(uStageHeight, 0.45, 2.2), galaxyPreset);
  float galaxyMotion = mix(1.0, clamp(uStageResponse, 0.25, 2.4), galaxyPreset);
  float galaxyNebula = mix(1.0, clamp(uStageWave, 0.15, 2.6), galaxyPreset);
  float galaxySparkle = mix(1.0, clamp(uStageTrigger, 0.1, 2.8), galaxyPreset);
  float galaxyLayer = mix(1.0, clamp(uStageLayerHeight, 0.25, 2.8), galaxyPreset);
  vec3 pos = position;
  float alpha = 0.0;
  float glow = 0.0;
  vec3 color = mix(uColorLow, uColorMid, aBand);

  if (aKind < 0.5) {
    if (galaxyPreset > 0.5) {
      float starSpin = t * (0.016 + aSeed * 0.024) * playMix * driftPower * galaxyMotion;
      pos.xz = rotate2d(starSpin) * pos.xz;
      pos.xz *= 0.74 + galaxyDepth * 0.28;
      pos.y *= 0.52 + galaxyLayer * 0.48;
      pos.z = (pos.z + 2.1) * (0.72 + galaxyDepth * 0.32) - 2.1;
      pos.xyz *= 1.0 + (uBass * 0.045 + uBeat * 0.068) * motion * galaxySparkle;
      pos.y += sin(t * 0.52 * galaxyMotion + aSeed * 9.0) * (0.04 + galaxyLayer * 0.02 + uTreble * 0.08 * galaxySparkle);
      pos.z += sin(t * 0.18 * galaxyMotion + aSeed * 12.0) * 0.42 * (0.72 + galaxyDepth * 0.18);
      color = mix(uColorMid, uColorHigh, twinkle * 0.86);
      color = mix(color, uColorPulse, uBeat * 0.2 * galaxySparkle);
      glow = twinkle * (0.72 + galaxySparkle * 0.28) + uBeat * 0.34 * galaxySparkle;
      alpha = 0.1 + twinkle * 0.46 * (0.72 + galaxySparkle * 0.28) + uVolume * 0.14;
    } else if (cinematicPreset > 0.5) {
      pos.x = (aBand - 0.5) * 8.6 + sin(t * 0.28 + aSeed * TAU) * 0.28;
      pos.y = -1.18 + pow(aLane, 1.7) * 2.75 + sin(t * 0.36 + aSeed * 8.0) * 0.08;
      pos.z = -4.35 + aLane * 3.85 + uBeat * 0.28;
      color = mix(uColorShadow, uColorLine, aLane);
      color = mix(color, uColorPulse, twinkle * 0.24 + uBeat * 0.28);
      glow = twinkle * 0.18 + uBeat * 0.34;
      alpha = 0.035 + twinkle * 0.08 + uBeat * 0.08;
    } else if (djPreset > 0.5) {
      float scanRow = floor(aLane * 14.0) / 13.0;
      pos.x = (aBand - 0.5) * 8.4;
      pos.y = -1.06 + scanRow * 0.22 + sin(t * 1.6 + aBand * TAU) * 0.04;
      pos.z = -4.05 + scanRow * 4.65 + fract(aSeed + t * 0.08) * 0.18;
      color = mix(uColorLine, uColorPulse, twinkle * 0.45 + uTreble * 0.22);
      glow = twinkle * 0.24 + uBeat * 0.26;
      alpha = 0.04 + twinkle * 0.11 + uVolume * 0.06;
    } else if (lyricPreset > 0.5) {
      pos.x = (aBand - 0.5) * 5.4 + sin(aSeed * TAU) * 0.16;
      pos.y = -0.82 + sin(t * 0.18 + aSeed * 5.0) * 0.04;
      pos.z = -3.12 + aLane * 0.82;
      color = mix(uColorShadow, uColorHalo, 0.42 + twinkle * 0.12);
      glow = twinkle * 0.08 + uVolume * 0.08;
      alpha = 0.026 + twinkle * 0.045;
    } else {
      float stageSpin = t * (0.006 + aSeed * 0.01) * playMix;
      pos.xz = rotate2d(stageSpin) * pos.xz;
      pos.xyz *= 0.72 + (uBass * 0.022 + uBeat * 0.035) * motion;
      pos.y += sin(t * 0.34 + aSeed * 8.0) * (0.035 + uTreble * 0.035);
      color = mix(uColorMid, uColorHigh, twinkle * 0.56);
      color = mix(color, uColorPulse, uBeat * 0.16);
      glow = twinkle * 0.54 + uBeat * 0.2;
      alpha = 0.075 + twinkle * 0.23 + uVolume * 0.08;
    }
  } else if (aKind < 1.5) {
    float ribbonEnergy = smoothstep(0.08, 1.0, energy + uBeat * 0.38);
    if (galaxyPreset > 0.5) {
      float flow = t * (0.17 + aSeed * 0.052) * playMix * driftPower * galaxyMotion;
      float armWave = sin(aLane * TAU * 4.2 + flow + aSeed * 4.0) * (0.14 + uMid * 0.32) * galaxyNebula;
      pos.xz = rotate2d(flow * 0.34 + uBass * 0.08) * pos.xz;
      pos.xz *= 0.76 + galaxyDepth * 0.28;
      pos.y = pos.y * (0.58 + galaxyLayer * 0.38) + armWave * (0.72 + galaxyLayer * 0.28) + (uBeat * 0.2 + uBass * 0.12) * sin(aLane * TAU) * galaxyNebula * (0.7 + galaxyLayer * 0.3);
      pos.z = (pos.z + 1.45) * (0.74 + galaxyDepth * 0.3) - 1.45;
      pos.xyz *= 1.0 + ribbonEnergy * 0.11 * motion * galaxyNebula;
      color = mix(uColorLow, uColorHalo, aLane);
      color = mix(color, uColorHigh, twinkle * 0.2 * galaxySparkle);
      glow = ribbonEnergy * galaxyNebula + twinkle * 0.26 * galaxySparkle + 0.12;
      alpha = 0.08 + ribbonEnergy * 0.38 * galaxyNebula + twinkle * 0.18 * galaxySparkle;
    } else if (cinematicPreset > 0.5) {
      pos.x = (aBand - 0.5) * 8.2;
      pos.y = -1.12 + aLane * 2.75 + sin(aBand * PI * 2.0 + t * 0.38) * (0.14 + uMid * 0.18);
      pos.z = -4.18 + aLane * 3.8 + ribbonEnergy * 0.38;
      color = mix(uColorShadow, uColorHalo, aLane);
      color = mix(color, uColorPulse, ribbonEnergy * 0.34);
      glow = ribbonEnergy * 0.54 + uBeat * 0.32;
      alpha = 0.055 + ribbonEnergy * 0.16 + twinkle * 0.06;
    } else if (djPreset > 0.5) {
      float gridRow = floor(aLane * 10.0) / 9.0;
      pos.x = fract(aBand + t * 0.06 + aSeed * 0.18) * 8.4 - 4.2;
      pos.y = -1.05 + gridRow * 0.24 + sin(t * 1.35 + aSeed * TAU) * 0.05;
      pos.z = -4.1 + gridRow * 4.65;
      color = mix(uColorLine, uColorPulse, ribbonEnergy * 0.56 + twinkle * 0.2);
      glow = ribbonEnergy * 0.46 + twinkle * 0.18;
      alpha = 0.045 + ribbonEnergy * 0.18 + twinkle * 0.07;
    } else if (lyricPreset > 0.5) {
      pos.x = (aBand - 0.5) * 5.2;
      pos.y = -0.92 + sin(aBand * PI + t * 0.16) * 0.05;
      pos.z = -3.16 + aLane * 0.64;
      color = mix(uColorShadow, uColorHalo, 0.5 + ribbonEnergy * 0.16);
      glow = ribbonEnergy * 0.16 + twinkle * 0.08;
      alpha = 0.025 + ribbonEnergy * 0.055;
    } else {
      float flow = t * (0.08 + aSeed * 0.03) * playMix;
      float haloWave = sin(aLane * TAU * 2.0 + flow + aSeed * 3.0) * (0.08 + uMid * 0.16);
      pos.xz = rotate2d(flow * 0.18 + uBass * 0.04) * pos.xz;
      pos.xyz *= 0.76 + ribbonEnergy * 0.055 * motion;
      pos.y = pos.y * 0.72 + haloWave + uBeat * 0.06;
      color = mix(uColorLow, uColorHalo, aLane);
      color = mix(color, uColorHigh, twinkle * 0.14);
      glow = ribbonEnergy * 0.58 + twinkle * 0.18;
      alpha = 0.06 + ribbonEnergy * 0.18 + twinkle * 0.08;
    }
  } else if (aKind < 2.5) {
    float columnEnergy = bandEnergy(aBand);
    float bandPulse = smoothstep(0.04, 1.0, columnEnergy + uBeat * (0.25 + cinematicPreset * 0.25 + djPreset * 0.18));
    float x = (aBand - 0.5) * 6.7;
    float y = (aLane - 0.5) * (1.0 + bandPulse * 2.8);
    float z = -2.3 + sin(aBand * PI) * 1.0 + bandPulse * 0.5;
    float djColumnMask = 1.0;
    float djColumnCap = 0.0;

    if (galaxyPreset > 0.5) {
      vec3 galaxyCluster = pos;
      float clusterDrift = t * (0.045 + aSeed * 0.035) * playMix * galaxyMotion;
      float clusterBreath = 1.0 + bandPulse * 0.075 * galaxySparkle + uBass * 0.025;
      galaxyCluster.xz = rotate2d(clusterDrift + aSeed * 0.18) * galaxyCluster.xz;
      galaxyCluster.xz *= (0.78 + galaxyDepth * 0.31) * clusterBreath;
      galaxyCluster.y =
        galaxyCluster.y * (0.48 + galaxyLayer * 0.24 + galaxyNebula * 0.1) +
        sin(t * 0.42 * galaxyMotion + aSeed * 9.0) * (0.04 + galaxyLayer * 0.02 + uMid * 0.12) * galaxyNebula +
        bandPulse * 0.2 * galaxySparkle;
      galaxyCluster.z =
        (galaxyCluster.z + 2.0) * (0.76 + galaxyDepth * 0.27) -
        2.0 +
        sin(t * 0.18 * galaxyMotion + aSeed * 13.0) * 0.16 * galaxyNebula;
      x = galaxyCluster.x;
      y = galaxyCluster.y;
      z = galaxyCluster.z;
    } else if (cinematicPreset > 0.5) {
      float terrainWave = sin(aBand * PI * 3.0 + t * 0.42) * 0.18 + sin(aLane * PI * 5.0 - t * 0.34) * 0.14;
      float lift = pow(bandPulse, 1.25) * (0.52 + aLane * 1.58);
      x = (aBand - 0.5) * 8.2;
      y = -1.28 + terrainWave + lift;
      z = -4.15 + aLane * 4.9 + bandPulse * 0.42;
    } else if (djPreset > 0.5) {
      float gridBand = floor(aBand * 32.0) / 31.0;
      float heightStep = floor(aLane * 20.0) / 19.0;
      float depthLane = floor(fract(aSeed * 19.0) * 4.0) / 3.0;
      float lowColumnEnergy = mix(uBass, uMid, smoothstep(0.16, 0.58, gridBand));
      float highColumnEnergy = mix(uMid, uTreble, smoothstep(0.48, 1.0, gridBand));
      float gridEnergy = mix(lowColumnEnergy, highColumnEnergy, smoothstep(0.42, 0.82, gridBand));
      float columnPulse = smoothstep(0.04, 1.0, gridEnergy + uBeat * (0.18 + (1.0 - gridBand) * 0.16));
      float columnHeight = clamp(0.10 + pow(columnPulse, 0.72) * 0.86 + uBeat * 0.08, 0.08, 1.0);
      float columnBody = 1.0 - smoothstep(columnHeight, columnHeight + 0.035, heightStep);
      float columnCap = 1.0 - smoothstep(0.0, 0.055, abs(heightStep - columnHeight));
      x = (gridBand - 0.5) * 8.4 + (fract(aSeed * 7.0) - 0.5) * 0.045;
      y = -1.24 + heightStep * (3.1 + uBeat * 0.2);
      z = -3.95 + depthLane * 0.36 + sin(gridBand * PI) * 0.28;
      djColumnMask = max(columnBody, columnCap * 0.78);
      djColumnCap = columnCap;
      bandPulse = columnPulse;
    } else if (lyricPreset > 0.5) {
      x = (aBand - 0.5) * 5.3;
      y = -0.88 + sin(aBand * PI * 2.0 + t * 0.16) * 0.06 + bandPulse * 0.12;
      z = -3.08 + aLane * 0.76;
    } else {
      x = (aBand - 0.5) * 6.0;
      y = (aLane - 0.48) * (0.78 + bandPulse * 1.42);
      z = -2.65 + sin(aBand * PI) * 0.62 + bandPulse * 0.28;
    }

    pos = vec3(x, y, z);
    if (djPreset > 0.5) {
      color = mix(uColorShadow, uColorLine, djColumnMask * 0.82 + bandPulse * 0.12);
      color = mix(color, uColorPulse, max(djColumnCap, uBeat * 0.22));
      glow = (0.10 + bandPulse * 0.95) * djColumnMask + djColumnCap * 0.58 + twinkle * 0.08;
      alpha = 0.035 + djColumnMask * 0.48 + djColumnCap * 0.22;
    } else {
      vec3 stageLow = mix(uColorLow, uColorShadow, cinematicPreset * 0.44);
      vec3 stageMid = mix(uColorMid, uColorLine, cinematicPreset * 0.58);
      vec3 stageHigh = uColorHigh;
      stageMid = mix(stageMid, uColorHalo, galaxyPreset * 0.42);
      color = mix(stageLow, stageMid, aBand);
      color = mix(color, stageHigh, bandPulse * 0.52);
      glow = bandPulse * (1.0 + cinematicPreset * 0.36) + twinkle * 0.18;
      alpha = 0.10 + bandPulse * 0.48 + twinkle * 0.08;
      alpha *= 1.0 + cinematicPreset * 0.18 - galaxyPreset * 0.18 - lyricPreset * 0.46;
    }
  } else if (aKind < 3.5) {
    float ringEnergy = max(uBeat, uBass * 0.58 + uVolume * 0.18);
    float angle = aLane * TAU;
    float ring = aBand;
    float radius = 1.15 + ring * 3.2 + ringEnergy * (0.65 + ring * 0.8) * stagePower;
    float wave = sin(angle * 2.0 + t * 1.2 + ring * 4.0) * (0.04 + uMid * 0.16);

    if (galaxyPreset > 0.5) {
      ringEnergy *= galaxySparkle;
      float cloudDrift = t * (0.06 + aSeed * 0.04) * playMix * galaxyMotion;
      float cloudPulse = 1.0 + ringEnergy * 0.06 + uTreble * 0.025 * galaxySparkle;
      pos.xz = rotate2d(cloudDrift + aSeed * 0.24) * pos.xz;
      pos.xz *= (0.76 + galaxyDepth * 0.3) * cloudPulse;
      pos.y =
        pos.y * (0.48 + galaxyLayer * 0.22 + galaxyNebula * 0.12) +
        sin(t * 0.78 * galaxyMotion + aSeed * TAU) * (0.035 + galaxyLayer * 0.015 + uTreble * 0.1) * galaxyNebula +
        ringEnergy * 0.18;
      pos.z =
        (pos.z + 1.95) * (0.78 + galaxyDepth * 0.26) -
        1.95 +
        sin(t * 0.22 * galaxyMotion + aSeed * 11.0) * 0.16 * galaxyNebula;
    } else if (cinematicPreset > 0.5) {
      radius = 0.92 + ring * 3.45 + ringEnergy * (1.0 + ring * 0.9);
      radius += ringEnergy * sin(t * 3.4 + ring * PI) * 0.58;
      pos.y = sin(angle) * radius * 0.24 + ringEnergy * 0.12;
    } else if (djPreset > 0.5) {
      radius = 0.7 + ring * 2.2 + ringEnergy * 0.9;
      pos.y = -1.02 + floor(aLane * 8.0) * 0.055 + ringEnergy * 0.22;
      pos.z = -3.65 + ring * 3.8;
    } else if (lyricPreset > 0.5) {
      radius = 1.15 + ring * 1.75 + ringEnergy * 0.2;
      pos.y = wave * 0.12 - 0.34;
    } else {
      radius = 1.05 + ring * 2.55 + ringEnergy * 0.56;
      pos.y = sin(angle) * radius * 0.13 + wave * 0.52;
    }

    if (galaxyPreset < 0.5) {
      pos.x = cos(angle) * radius;
      if (djPreset < 0.5) {
        pos.z = sin(angle) * radius - 2.0 - ring * 0.18;
      }
    }
    vec3 burstBase = mix(uColorRipple, uColorLine, max(cinematicPreset, djPreset) * 0.52);
    vec3 burstPeak = mix(uColorHalo, uColorHigh, cinematicPreset * 0.46);
    burstPeak = mix(burstPeak, uColorPulse, djPreset * 0.4);
    color = mix(burstBase, burstPeak, ringEnergy * 0.48 + ring * 0.2);
    glow = ringEnergy + twinkle * 0.16;
    alpha = 0.07 + ringEnergy * 0.42 + (1.0 - ring) * 0.08;
    alpha *= 1.0 + cinematicPreset * 0.18 - galaxyPreset * 0.22 - lyricPreset * 0.78;
  } else {
    float linePulse = max(uBeat * 0.75, energy * 0.45);
    float path = aLane * TAU * 2.0 + t * 0.35;
    pos.xz = rotate2d(t * 0.025 * driftPower + uBass * 0.08) * pos.xz;
    pos.y += sin(path + aSeed * 5.0) * (0.12 + uMid * 0.28);

    if (cinematicPreset > 0.5) {
      pos.x = (fract(aLane + t * 0.05 + aSeed * 0.12) - 0.5) * 8.6;
      pos.y = -0.96 + sin(aBand * PI * 2.0 + t * 0.7) * (0.18 + uMid * 0.18) + linePulse * 0.24;
      pos.z = -4.0 + aBand * 4.6;
    } else if (djPreset > 0.5) {
      float scan = fract(aLane + t * 0.1);
      pos.x = scan * 8.2 - 4.1;
      pos.y = -0.92 + floor(aBand * 9.0) * 0.09 + linePulse * 0.18;
      pos.z = -4.12 + aBand * 4.7;
    } else if (galaxyPreset > 0.5) {
      float comet = fract(aLane + t * 0.06 * galaxyMotion + aSeed * 0.28);
      float angle = aBand * TAU * 2.0 + comet * 1.4;
      float radius = (2.1 + aSeed * 3.4) * (0.74 + galaxyDepth * 0.3);
      pos.x = cos(angle) * radius;
      pos.y = (comet - 0.5) * 2.2 * galaxyNebula * (0.58 + galaxyLayer * 0.42) + sin(aLane * TAU * 3.0 + t * 0.34 * galaxyMotion) * 0.24 * galaxyNebula * (0.68 + galaxyLayer * 0.32);
      pos.z = sin(angle) * radius - 2.05 - comet * 0.52 * galaxySparkle;
    } else if (lyricPreset > 0.5) {
      pos.x = (aBand - 0.5) * 5.6;
      pos.y = -0.72 + sin(aBand * PI + t * 0.18) * 0.04;
      pos.z = -3.28 + aLane * 0.58;
    } else {
      pos.xyz *= 0.82 + linePulse * 0.04;
      pos.y *= 0.62;
      pos.z -= 0.18;
    }

    vec3 trailBase = mix(uColorLine, uColorShadow, djPreset * 0.28);
    vec3 trailPeak = mix(uColorPulse, uColorHigh, cinematicPreset * 0.4 + galaxyPreset * 0.28);
    color = mix(trailBase, trailPeak, 0.42 + linePulse * 0.48);
    glow = linePulse + twinkle * 0.2;
    alpha = 0.06 + linePulse * 0.34 + twinkle * 0.12;
    alpha *= 1.0 + galaxyPreset * 0.1 + djPreset * 0.16 - lyricPreset * 0.62;
  }

  float layerBoost = uLayerScale > 2.0 ? 0.55 : 1.0;
  vColor = color;
  vGlow = glow;
  float presetAlpha = 1.0 + galaxyPreset * 0.08 - lyricPreset * 0.28 + defaultPreset * 0.02;
  vAlpha = max(0.0, alpha) * presetAlpha * uLayerAlpha * layerBoost * (0.62 + uIntensity * 0.74);

  vec4 mvPosition = modelViewMatrix * vec4(pos, 1.0);
  float perspectiveScale = clamp(8.0 / max(1.0, -mvPosition.z), 0.45, 2.8);
  float audioSize = 1.0 + energy * 0.68 + uBeat * 0.52 + glow * 0.18;
  float pointSize = aSize * uLayerScale * uPixelRatio * perspectiveScale * audioSize * (0.78 + uIntensity * 0.58);
  gl_PointSize = clamp(pointSize, 1.0, 58.0);
  gl_Position = projectionMatrix * mvPosition;
}
`

const fragmentShaderSource = `
precision highp float;

uniform sampler2D uDotTex;
uniform float uLayerAlpha;

varying vec3 vColor;
varying float vAlpha;
varying float vGlow;

void main() {
  vec4 dot = texture2D(uDotTex, gl_PointCoord);
  if (dot.a < 0.018) {
    discard;
  }

  vec3 color = vColor * (0.72 + vGlow * 0.72);
  gl_FragColor = vec4(color, dot.a * vAlpha * uLayerAlpha);
}
`

function visualPalette(theme: MusicImmersiveTheme | string, mode: MusicVisualMode): VisualPalette {
  if (theme === 'animal-island') {
    return {
      low: '#ffd97b',
      mid: '#7fb6a6',
      high: '#fff8e9',
      pulse: '#bfe8d0',
      shadow: '#8d6b3b',
      line: '#ffe7ad',
      ripple: '#78d3a8',
      halo: '#fff2c7',
    }
  }

  if (theme === 'cinema') {
    return {
      low: '#ff5367',
      mid: '#7ad7c2',
      high: '#fff1bd',
      pulse: '#fff4d6',
      shadow: '#4a2a2f',
      line: '#f4d28a',
      ripple: '#ff7a86',
      halo: '#fff4d6',
    }
  }

  if (theme === 'galaxy') {
    return {
      low: '#73a7ff',
      mid: '#9cffdf',
      high: '#fff0b8',
      pulse: '#9cffdf',
      shadow: '#243d78',
      line: '#8fe9ff',
      ripple: '#6d83ff',
      halo: '#fff0b8',
    }
  }

  if (theme === 'neon') {
    return {
      low: '#ff4fd8',
      mid: '#00f5d4',
      high: '#eff7ff',
      pulse: '#ff4fd8',
      shadow: '#30145c',
      line: '#7df9ff',
      ripple: '#b76cff',
      halo: '#eff7ff',
    }
  }

  if (theme === 'sunset') {
    return {
      low: '#ff8a5c',
      mid: '#f4d28a',
      high: '#fff8df',
      pulse: '#ff705f',
      shadow: '#5c2330',
      line: '#ffd28a',
      ripple: '#ff9b72',
      halo: '#fff1bd',
    }
  }

  if (theme === 'midnight') {
    return {
      low: '#6f93c7',
      mid: '#9fb7d9',
      high: '#eef6ff',
      pulse: '#b8cdfa',
      shadow: '#17253e',
      line: '#9fb7d9',
      ripple: '#7898cb',
      halo: '#dceaff',
    }
  }

  if (mode === 'sleep') {
    return {
      low: '#86b7ff',
      mid: '#b5c7f1',
      high: '#ffffff',
      pulse: '#8fb3ff',
      shadow: '#1b2d4d',
      line: '#b8cdfa',
      ripple: '#86b7ff',
      halo: '#ffffff',
    }
  }

  if (mode === 'focus') {
    return {
      low: '#b6e6cf',
      mid: '#78d3c7',
      high: '#f5fff9',
      pulse: '#78d3c7',
      shadow: '#21483f',
      line: '#b6e6cf',
      ripple: '#6fd8c7',
      halo: '#f5fff9',
    }
  }

  return {
    low: '#ffb861',
    mid: '#2aa7d9',
    high: '#f9fcff',
    pulse: '#ffcf7a',
    shadow: '#24394f',
    line: '#b8f2ff',
    ripple: '#74a7ff',
    halo: '#fff0bd',
  }
}

function terrainFogColor(theme: MusicImmersiveTheme | string, mode: MusicVisualMode) {
  if (theme === 'animal-island') {
    return '#241b12'
  }
  if (theme === 'cinema') {
    return '#17080e'
  }
  if (theme === 'galaxy') {
    return '#07142d'
  }
  if (theme === 'neon') {
    return '#10061f'
  }
  if (theme === 'sunset') {
    return '#281018'
  }
  if (theme === 'midnight') {
    return '#07101d'
  }
  if (mode === 'sleep') {
    return '#081526'
  }
  if (mode === 'focus') {
    return '#0d211d'
  }
  return '#0b1624'
}

function stagePresetCode(preset: MusicVisualStagePreset) {
  const codes: Record<MusicVisualStagePreset, number> = {
    default: 0,
    galaxy: 1,
    cinematic: 2,
    dj: 3,
    lyric: 4,
  }
  return codes[preset] ?? 0
}

function spectrumStyleCode(style: MusicSpectrumStyle) {
  const codes: Record<MusicSpectrumStyle, number> = {
    bars: 0,
    mirror: 1,
    orbit: 2,
    particles: 3,
    ribbon: 4,
    none: 5,
  }
  return codes[style] ?? 0
}

function lineStyleCode(style: MusicLineStyle) {
  const codes: Record<MusicLineStyle, number> = {
    wave: 0,
    beam: 1,
    scan: 2,
    constellation: 3,
    none: 4,
  }
  return codes[style] ?? 0
}

function rippleStyleCode(style: MusicRippleStyle) {
  const codes: Record<MusicRippleStyle, number> = {
    rings: 0,
    water: 1,
    heartbeat: 2,
    halo: 3,
    none: 4,
  }
  return codes[style] ?? 0
}

function stageTuningValue(key: MusicStageTuningKey, min: number, max: number) {
  const rawValue = props.stageTuning?.[key] ?? DEFAULT_MUSIC_STAGE_TUNING[key]
  return clamp(rawValue, min, max)
}

function createSilentSmoothedEnergy(): SmoothedMusicEnergy {
  return {
    bass: 0,
    mid: 0,
    treble: 0,
    beat: 0,
    volume: 0,
    rhythm: 0,
  }
}

function terrainDensitySignature() {
  return props.stagePreset === 'dj' ? Math.round(stageTuningValue('density', 0.35, 1.7) * 100) : -1
}

function starDensitySignature() {
  return props.stagePreset === 'galaxy' ? Math.round(stageTuningValue('density', 0.35, 2) * 100) : -1
}

function pseudoRandom(seed: number) {
  const value = Math.sin(seed * 12.9898) * 43758.5453
  return value - Math.floor(value)
}

function createTerrainFluxTriggers(): TerrainFluxTriggerState[] {
  return [
    createTerrainFluxTrigger('pulse', 0.002, 0.008, 0.85, 0.24, 7.2),
    createTerrainFluxTrigger('snare', 0.092, 0.235, 0.6, 0.42, 5.8),
    createTerrainFluxTrigger('spark', 0.235, 0.42, 0.48, 0.56, 5.2),
  ]
}

function createTerrainFluxTrigger(
  kind: TerrainRippleKind,
  bandStartRatio: number,
  bandEndRatio: number,
  sensitivity: number,
  cooldownSeconds: number,
  strengthScale: number,
): TerrainFluxTriggerState {
  return {
    kind,
    bandStartRatio,
    bandEndRatio,
    sensitivity,
    cooldownSeconds,
    strengthScale,
    history: new Array(TERRAIN_FLUX_HISTORY_SIZE).fill(0),
    historyIndex: 0,
    smoothedFlux: 0,
    prevSmoothedFlux: 0,
    cooldownRemaining: 0,
    lastEnergy: 0,
    lastThreshold: 0,
  }
}

function resetTerrainRipples() {
  terrainRipples = []
  terrainRippleIndex = 0
  terrainRippleSeed = 0
  terrainFluxTriggers = createTerrainFluxTriggers()
  previousTerrainFrequencyData = null
  lastTerrainTriggerUpdateTime = 0
  lastTerrainBeatLevel = 0
  lastTerrainEnergyLevel = 0
  lastTerrainBassRippleTime = -999
  lastTerrainMidRippleTime = -999
  lastTerrainSparkRippleTime = -999
}

function updateTerrainRippleTriggers(
  time: number,
  frame: {
    frequencyData: Uint8Array | null
    playing: boolean
    subBass: number
    bass: number
    lowMid: number
    mid: number
    treble: number
    beat: number
    volume: number
    energy: number
    triggerTuning: number
    reducedMotion: boolean
  },
) {
  const delta =
    lastTerrainTriggerUpdateTime > 0
      ? clamp(time - lastTerrainTriggerUpdateTime, 0, 0.25)
      : 1 / 60
  lastTerrainTriggerUpdateTime = time

  if (!frame.playing) {
    if (frame.frequencyData) {
      updateTerrainPreviousFrequencyData(frame.frequencyData)
    }
    return
  }

  if (frame.frequencyData && frame.frequencyData.length >= 8) {
    for (const trigger of terrainFluxTriggers) {
      const strength = evaluateTerrainFluxTrigger(
        trigger,
        frame.frequencyData,
        delta,
        frame.reducedMotion,
        frame.triggerTuning,
      )
      if (strength > 0) {
        spawnTerrainRipple(time, strength, trigger.kind)
      }
    }
    updateTerrainPreviousFrequencyData(frame.frequencyData)
    return
  }

  updateTerrainFallbackRippleTriggers(time, frame)
}

function evaluateTerrainFluxTrigger(
  trigger: TerrainFluxTriggerState,
  data: Uint8Array,
  delta: number,
  reducedMotion: boolean,
  triggerTuning: number,
) {
  const fluxScore = terrainFluxScore(trigger, data)
  trigger.smoothedFlux += (fluxScore - trigger.smoothedFlux) * 0.4
  trigger.history[trigger.historyIndex] = trigger.smoothedFlux
  trigger.historyIndex = (trigger.historyIndex + 1) % trigger.history.length

  let avgFlux = 0
  for (const value of trigger.history) {
    avgFlux += value
  }
  avgFlux /= Math.max(1, trigger.history.length)

  let variance = 0
  for (const value of trigger.history) {
    variance += Math.pow(value - avgFlux, 2)
  }
  const fluxStdDev = Math.sqrt(variance / Math.max(1, trigger.history.length))
  const effectiveSensitivity = clamp(trigger.sensitivity * triggerTuning, 0.08, 1.8)
  const thresholdMultiplier = Math.max(0.1, 5 - effectiveSensitivity * 4)
  const adaptiveThreshold = Math.max(0.01, avgFlux + fluxStdDev * thresholdMultiplier)
  const isPeak =
    trigger.prevSmoothedFlux > adaptiveThreshold &&
    trigger.prevSmoothedFlux >= trigger.smoothedFlux &&
    trigger.prevSmoothedFlux - trigger.smoothedFlux > 0.0001

  trigger.cooldownRemaining = Math.max(0, trigger.cooldownRemaining - delta)
  trigger.lastEnergy = trigger.smoothedFlux * 10
  trigger.lastThreshold = adaptiveThreshold * 10

  let strength = 0
  if (trigger.cooldownRemaining <= 0 && isPeak) {
    strength = clamp01(trigger.prevSmoothedFlux * trigger.strengthScale * (0.72 + triggerTuning * 0.38))
    trigger.cooldownRemaining = (trigger.cooldownSeconds * (reducedMotion ? 1.35 : 1)) / Math.sqrt(triggerTuning)
  }

  trigger.prevSmoothedFlux = trigger.smoothedFlux
  return strength
}

function terrainFluxScore(trigger: TerrainFluxTriggerState, data: Uint8Array) {
  if (!previousTerrainFrequencyData || previousTerrainFrequencyData.length !== data.length) {
    return 0
  }

  const [start, end] = terrainFluxBand(data.length, trigger)
  let flux = 0
  let count = 0
  for (let index = start; index <= end; index += 1) {
    const value = (data[index] ?? 0) / 255
    const previous = previousTerrainFrequencyData[index] ?? 0
    const diff = value - previous
    if (diff > 0.01) {
      flux += diff
    }
    count += 1
  }

  return count > 0 ? flux / count : 0
}

function terrainFluxBand(dataLength: number, trigger: TerrainFluxTriggerState): [number, number] {
  const maxIndex = Math.max(0, dataLength - 1)
  const minWidth = trigger.kind === 'pulse' ? 2 : trigger.kind === 'snare' ? 8 : 10
  const rawStart = Math.floor(dataLength * trigger.bandStartRatio)
  const rawEnd = Math.ceil(dataLength * trigger.bandEndRatio)
  const start = clamp(trigger.kind === 'pulse' ? Math.max(1, rawStart) : rawStart, 0, maxIndex)
  const end = clamp(Math.max(start + minWidth - 1, rawEnd), start, maxIndex)
  return [start, end]
}

function updateTerrainPreviousFrequencyData(data: Uint8Array) {
  if (!previousTerrainFrequencyData || previousTerrainFrequencyData.length !== data.length) {
    previousTerrainFrequencyData = new Float32Array(data.length)
  }

  for (let index = 0; index < data.length; index += 1) {
    previousTerrainFrequencyData[index] = (data[index] ?? 0) / 255
  }
}

function updateTerrainFallbackRippleTriggers(
  time: number,
  frame: {
    subBass: number
    bass: number
    lowMid: number
    mid: number
    treble: number
    beat: number
    volume: number
    energy: number
    triggerTuning: number
    reducedMotion: boolean
  },
) {
  const triggerTuning = clamp(frame.triggerTuning, 0.25, 2.4)
  const beatRise = frame.beat - lastTerrainBeatLevel
  const energyRise = frame.energy - lastTerrainEnergyLevel
  const bassPush = clamp01(frame.subBass * 0.5 + frame.bass * 0.34 + frame.beat * 0.28)
  const midPush = clamp01(frame.lowMid * 0.38 + frame.mid * 0.48 + frame.volume * 0.18)
  const sparkPush = clamp01(frame.treble * 0.56 + frame.mid * 0.22 + frame.beat * 0.16)
  const cooldownScale = 1 / Math.sqrt(triggerTuning)
  const thresholdScale = 1 / triggerTuning
  const bassCooldown = (frame.reducedMotion ? 0.36 : 0.2) * cooldownScale
  const midCooldown = (frame.reducedMotion ? 0.64 : 0.4) * cooldownScale
  const sparkCooldown = (frame.reducedMotion ? 0.82 : 0.52) * cooldownScale
  const bassTriggered =
    (frame.beat > 0.36 * thresholdScale && beatRise > 0.055 * thresholdScale) ||
    (bassPush > 0.6 * thresholdScale && energyRise > 0.024 * thresholdScale)
  const midTriggered = midPush > 0.54 * thresholdScale && energyRise > 0.018 * thresholdScale
  const sparkTriggered =
    sparkPush > 0.58 * thresholdScale && (frame.treble - frame.mid * 0.22 > 0.22 * thresholdScale || beatRise > 0.04)
  const strengthScale = 0.72 + triggerTuning * 0.28

  if (bassTriggered && time - lastTerrainBassRippleTime > bassCooldown) {
    const rippleCount = !frame.reducedMotion && (frame.beat > 0.7 || bassPush > 0.72) ? 2 : 1
    for (let index = 0; index < rippleCount; index += 1) {
      spawnTerrainRipple(time, clamp01(bassPush * strengthScale * (index === 0 ? 1.16 : 0.78)), 'pulse')
    }
    lastTerrainBassRippleTime = time
  }

  if (midTriggered && time - lastTerrainMidRippleTime > midCooldown) {
    spawnTerrainRipple(time, clamp01(midPush * strengthScale * 0.92), 'snare')
    lastTerrainMidRippleTime = time
  }

  if (sparkTriggered && time - lastTerrainSparkRippleTime > sparkCooldown) {
    spawnTerrainRipple(time, clamp01(sparkPush * strengthScale * 0.72), 'spark')
    lastTerrainSparkRippleTime = time
  }

  lastTerrainBeatLevel = frame.beat
  lastTerrainEnergyLevel = frame.energy
}

function spawnTerrainRipple(time: number, strength: number, kind: TerrainRippleKind) {
  const safeStrength = clamp(strength, 0, kind === 'pulse' ? 1.08 : 1)
  if (safeStrength <= 0.08) {
    return
  }

  const seed = terrainRippleSeed + 1
  terrainRippleSeed = seed
  const angle = pseudoRandom(seed * 13.17 + time * 0.19) * Math.PI * 2
  const radiusNoise = pseudoRandom(seed * 7.91 + time * 0.37)
  const ring =
    kind === 'pulse'
      ? Math.pow(radiusNoise, 0.72) * 0.62
      : kind === 'snare'
        ? 0.22 + radiusNoise * 0.68
        : 0.48 + radiusNoise * 0.48
  const triggerRadius = kind === 'pulse' ? 5.45 : kind === 'snare' ? 5.55 : 5.65
  const ripple: TerrainRipple = {
    x: Math.cos(angle) * ring * triggerRadius,
    z: Math.sin(angle) * ring * triggerRadius,
    startedAt: time,
    strength: safeStrength,
    speedScale: terrainRippleSpeedScale(kind, safeStrength),
    kind,
    seed,
  }

  if (terrainRipples.length < TERRAIN_RIPPLE_LIMIT) {
    terrainRipples.push(ripple)
  } else {
    terrainRipples[terrainRippleIndex] = ripple
  }
  terrainRippleIndex = (terrainRippleIndex + 1) % TERRAIN_RIPPLE_LIMIT
}

function activeTerrainRipples(time: number, reducedMotion: boolean, waveTuning: number): ActiveTerrainRipple[] {
  const maxAge = (reducedMotion ? 3.25 : 3.65) * (0.86 + waveTuning * 0.14)
  const ripples: ActiveTerrainRipple[] = []
  for (const ripple of terrainRipples) {
    const age = time - ripple.startedAt
    if (age < 0 || age > maxAge) {
      continue
    }

    const speed =
      (ripple.kind === 'spark' ? 4.45 : ripple.kind === 'snare' ? 4.15 : 3.05) *
      (0.9 + waveTuning * 0.08) *
      ripple.speedScale
    const radius = age * speed
    const speedWidth = 0.94 + ripple.speedScale * 0.06
    const width = (ripple.kind === 'spark' ? 0.12 : ripple.kind === 'snare' ? 0.16 : 0.34) * (0.76 + waveTuning * 0.24) * speedWidth
    const fadeDistance =
      (ripple.kind === 'spark' ? 2.65 : ripple.kind === 'snare' ? 3.2 : 5.15) *
      (0.82 + waveTuning * 0.18) *
      (0.9 + ripple.speedScale * 0.1)
    const fade = Math.exp(-radius / fadeDistance) * (1 - smoothstep(maxAge * 0.76, maxAge, age))
    ripples.push({ ...ripple, radius, width, fade })
  }
  return ripples
}

function terrainRippleSpeedScale(kind: TerrainRippleKind, strength: number) {
  const triggerTuning = stageTuningValue('trigger', 0.25, 2.4)
  const rhythmDrive = clamp01(smoothedEnergy.rhythm * 0.42 + smoothedEnergy.volume * 0.18 + strength * 0.16)
  const kindDrive =
    kind === 'pulse'
      ? clamp01(smoothedEnergy.bass * 0.52 + smoothedEnergy.beat * 0.34 + smoothedEnergy.volume * 0.14)
      : kind === 'snare'
        ? clamp01(smoothedEnergy.mid * 0.42 + smoothedEnergy.beat * 0.3 + smoothedEnergy.volume * 0.2 + strength * 0.08)
        : clamp01(smoothedEnergy.treble * 0.48 + smoothedEnergy.beat * 0.26 + smoothedEnergy.volume * 0.14 + strength * 0.12)
  const musicDrive = clamp01(rhythmDrive * 0.46 + kindDrive * 0.54)
  const tuningDrive = 0.76 + triggerTuning * 0.18
  const speedScale = 0.82 + musicDrive * tuningDrive

  return clamp(speedScale, props.reducedMotion ? 0.74 : 0.8, props.reducedMotion ? 1.22 : 1.58)
}

function terrainRippleInfluence(cell: TerrainCell, ripples: ActiveTerrainRipple[]): TerrainRippleInfluence {
  let lift = 0
  let glow = 0
  let sharp = 0

  for (const ripple of ripples) {
    const distance = Math.hypot(cell.x - ripple.x, cell.z - ripple.z)
    const delta = distance - ripple.radius
    const wave = Math.exp(-(delta * delta) / ripple.width) * ripple.fade * ripple.strength
    const localFade = 1 - cell.edgeFade * 0.18

    if (ripple.kind === 'spark') {
      sharp += wave * 0.9 * localFade
      glow += wave * 0.62 * localFade
      lift += wave * 0.16 * localFade
    } else if (ripple.kind === 'snare') {
      sharp += wave * 0.72 * localFade
      glow += wave * 0.76 * localFade
      lift += wave * 0.2 * localFade
    } else {
      glow += wave * 0.94 * localFade
      lift += wave * 0.42 * localFade
    }
  }

  return {
    lift: clamp01(lift),
    glow: clamp01(glow),
    sharp: clamp01(sharp),
  }
}

function degreesToRadians(value: number) {
  return (Number.isFinite(value) ? value : 0) * Math.PI / 180
}

function clamp(value: number, min: number, max: number) {
  if (!Number.isFinite(value)) {
    return min
  }

  return Math.min(max, Math.max(min, value))
}

function clamp01(value: number) {
  return clamp(value, 0, 1)
}

function smoothstep(edge0: number, edge1: number, value: number) {
  const nextValue = clamp01((value - edge0) / Math.max(0.000001, edge1 - edge0))
  return nextValue * nextValue * (3 - 2 * nextValue)
}
</script>

<template>
  <canvas ref="canvas" class="music-webgl-starfield-canvas" aria-hidden="true" />
</template>
