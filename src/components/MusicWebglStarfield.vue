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
  PerspectiveCamera,
  Points,
  Scene,
  ShaderMaterial,
  Texture,
  Vector2,
  Vector4,
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
type MusicStageTuningKey = Exclude<keyof MusicStageTuning, 'centerPulse'>
type ThreeModule = typeof import('three')
type VisualKind = 'dust' | 'nebula' | 'stage' | 'burst' | 'trail'
type VisualKindCounts = Record<VisualKind, number>

const DJ_TERRAIN_CENTER_Z = -3.28
const DJ_TERRAIN_GROUND_Y = -1.86
const TERRAIN_FLUX_HISTORY_SIZE = 40
const TERRAIN_RIPPLE_LIMIT = 12
const TERRAIN_CENTER_PULSE_SPEED = 24
const TERRAIN_PULSE_SPEED = 3.05
const TERRAIN_PULSE_WIDTH = 0.34
const TERRAIN_PULSE_FADE_DISTANCE = 5.15
const TERRAIN_KICK_MIN_IMPULSE = 0.46
const TERRAIN_KICK_TARGET_DECAY_RATE = 10
const TERRAIN_KICK_RESPONSE_RATE = 26

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

interface TerrainMaterialPalette {
  baseDeep: string
  baseLift: string
  fog: string
  coolCore: string
  coolEdge: string
  warmCore: string
  warmEdge: string
  ripple: string
  spark: string
  rim: string
  glowIntensity: number
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
  uTime: { value: number }
  uBass: { value: number }
  uMid: { value: number }
  uTreble: { value: number }
  uBeat: { value: number }
  uVolume: { value: number }
  uSubBass: { value: number }
  uLowMid: { value: number }
  uHighMid: { value: number }
  uEnergy: { value: number }
  uTriggerEnergy: { value: number }
  uKickEnvelope: { value: number }
  uIntensity: { value: number }
  uReducedMotion: { value: number }
  uMaxHeight: { value: number }
  uBaseHeight: { value: number }
  uResponse: { value: number }
  uWave: { value: number }
  uColorLow: { value: Color }
  uColorMid: { value: Color }
  uColorHigh: { value: Color }
  uColorPulse: { value: Color }
  uColorShadow: { value: Color }
  uBaseColor1: { value: Color }
  uBaseColor2: { value: Color }
  uCoolCore: { value: Color }
  uCoolEdge: { value: Color }
  uWarmCore: { value: Color }
  uWarmEdge: { value: Color }
  uRippleColor: { value: Color }
  uSparkColor: { value: Color }
  uRimColor: { value: Color }
  uGlowIntensity: { value: number }
  uWarmth: { value: number }
  uBrightness: { value: number }
  uPresence: { value: number }
  uBrilliance: { value: number }
  uAir: { value: number }
  uSharpness: { value: number }
  uRippleCount: { value: number }
  uRipplePos: { value: Vector2[] }
  uRippleData: { value: Vector4[] }
  uRippleKind: { value: number[] }
  uCenterPulseData: { value: Vector4 }
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
    stagePreset: 'dj',
    spectrumStyle: 'orbit',
    lineStyle: 'scan',
    rippleStyle: 'heartbeat',
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
let terrainInfoAttribute: InstancedBufferAttribute | null = null
let terrainBandAttribute: InstancedBufferAttribute | null = null
let terrainCells: TerrainCell[] = []
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
let terrainKickEnvelope = 0
let terrainKickTarget = 0
let lastTerrainKickEnvelopeTime = 0
let terrainCenterPulseStartedAt = -999
let terrainCenterPulseStrength = 0
let terrainCenterPulseSpeedScale = 1
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

function createTerrainUniforms(three: ThreeModule): TerrainUniforms {
  return {
    uOpacity: { value: terrainLayerOpacity(props.stagePreset) },
    uFogColor: { value: new three.Color('#050812') },
    uTime: { value: 0 },
    uBass: { value: 0 },
    uMid: { value: 0 },
    uTreble: { value: 0 },
    uBeat: { value: 0 },
    uVolume: { value: 0 },
    uSubBass: { value: 0 },
    uLowMid: { value: 0 },
    uHighMid: { value: 0 },
    uEnergy: { value: 0 },
    uTriggerEnergy: { value: 0 },
    uKickEnvelope: { value: 0 },
    uIntensity: { value: 0.72 },
    uReducedMotion: { value: 0 },
    uMaxHeight: { value: terrainMaxHeight(props.stagePreset) },
    uBaseHeight: { value: 0.064 },
    uResponse: { value: 1 },
    uWave: { value: 1 },
    uColorLow: { value: new three.Color('#74a7ff') },
    uColorMid: { value: new three.Color('#8edcff') },
    uColorHigh: { value: new three.Color('#f6f0b8') },
    uColorPulse: { value: new three.Color('#ff7a86') },
    uColorShadow: { value: new three.Color('#050812') },
    uBaseColor1: { value: new three.Color('#07111f') },
    uBaseColor2: { value: new three.Color('#10253a') },
    uCoolCore: { value: new three.Color('#3ad9ff') },
    uCoolEdge: { value: new three.Color('#2189d4') },
    uWarmCore: { value: new three.Color('#ffc46c') },
    uWarmEdge: { value: new three.Color('#ff7f5a') },
    uRippleColor: { value: new three.Color('#77d6ff') },
    uSparkColor: { value: new three.Color('#f8fbff') },
    uRimColor: { value: new three.Color('#d7faff') },
    uGlowIntensity: { value: 1.08 },
    uWarmth: { value: 0 },
    uBrightness: { value: 0 },
    uPresence: { value: 0 },
    uBrilliance: { value: 0 },
    uAir: { value: 0 },
    uSharpness: { value: 0 },
    uRippleCount: { value: 0 },
    uRipplePos: {
      value: Array.from({ length: TERRAIN_RIPPLE_LIMIT }, () => new three.Vector2(999, 999)),
    },
    uRippleData: {
      value: Array.from({ length: TERRAIN_RIPPLE_LIMIT }, () => new three.Vector4(0, 1, 0, 0)),
    },
    uRippleKind: { value: new Array(TERRAIN_RIPPLE_LIMIT).fill(0) },
    uCenterPulseData: { value: new three.Vector4(0, 1, 0, 0) },
  }
}

function createTerrainLayer(three: ThreeModule) {
  if (!sceneGroup || props.stagePreset !== 'dj') {
    terrainCells = []
    return
  }

  terrainCells = createTerrainCells(props.stagePreset, props.reducedMotion, stageTuningValue('density', 0.35, 1.7))
  resetTerrainRipples()
  terrainGeometry = new three.BoxGeometry(1, 1, 1)
  terrainInfoAttribute = new three.InstancedBufferAttribute(new Float32Array(terrainCells.length * 4), 4)
  terrainBandAttribute = new three.InstancedBufferAttribute(new Float32Array(terrainCells.length * 4), 4)
  terrainGeometry.setAttribute('aTerrainInfo', terrainInfoAttribute)
  terrainGeometry.setAttribute('aTerrainBand', terrainBandAttribute)
  terrainUniforms = createTerrainUniforms(three)
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
  const terrainDummy = new three.Object3D()
  for (let index = 0; index < terrainCells.length; index += 1) {
    const cell = terrainCells[index]
    const visibleFade = 1 - cell.edgeFade * 0.34
    const innerFade = 0.5 + smoothstep(0.02, 0.2, cell.ring) * 0.5
    const radialGate = innerFade * visibleFade
    const currentSize = cell.size * (0.68 + radialGate * 0.32)

    terrainInfoAttribute.setXYZW(index, cell.ring, cell.angle, cell.spiral, cell.seed)
    terrainBandAttribute.setXYZW(index, cell.band, cell.lane, cell.edgeFade, currentSize)
    terrainDummy.position.set(cell.x, DJ_TERRAIN_GROUND_Y + 0.5, cell.z)
    terrainDummy.scale.set(currentSize, 1, currentSize)
    terrainDummy.updateMatrix()
    terrainMesh.setMatrixAt(index, terrainDummy.matrix)
  }
  terrainInfoAttribute.needsUpdate = true
  terrainBandAttribute.needsUpdate = true
  terrainMesh.instanceMatrix.needsUpdate = true
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
  terrainInfoAttribute = null
  terrainBandAttribute = null
  terrainCells = []
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
  const footprint = preset === 'dj' ? 12.2 : 7.4
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
  if (!terrainMesh || !terrainUniforms) {
    return
  }

  const bass = clamp01(smoothedEnergy.bass)
  const mid = clamp01(smoothedEnergy.mid)
  const treble = clamp01(smoothedEnergy.treble)
  const beat = clamp01(smoothedEnergy.beat)
  const volume = clamp01(smoothedEnergy.volume)
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
  const terrainPalette = terrainMaterialPalette(props.theme, props.mode)
  const lowMid = clamp01(bass * 0.28 + mid * 0.72)
  const highMid = clamp01(mid * 0.42 + treble * 0.58)
  const subBass = clamp01(bass * 0.56 + beat * 0.2)
  const energy = clamp01(volume * 0.46 + bass * 0.22 + mid * 0.18 + treble * 0.14)
  const triggerEnergy = clamp01(subBass * 0.34 + bass * 0.32 + beat * 0.22 + volume * 0.12)
  const presence = clamp01(treble * 0.46 + highMid * 0.34 + beat * 0.12 + volume * 0.08)
  const brilliance = clamp01(treble * 0.62 + beat * 0.18 + energy * 0.2)
  const air = clamp01(treble * 0.56 + highMid * 0.22 + (1 - bass) * volume * 0.12 + energy * 0.1)
  const lowWeight = subBass + bass + lowMid + mid * 0.42
  const highWeight = presence + brilliance + air + 0.001
  const warmth = clamp01(lowWeight / Math.max(0.001, lowWeight + highWeight) + beat * 0.08)
  const brightness = clamp01((presence + brilliance + air) / 3 * 0.78 + energy * 0.24 + triggerEnergy * 0.08)
  const sharpness = clamp01(treble * 0.42 + beat * 0.28 + Math.abs(treble - mid) * 0.2 + triggerEnergy * 0.1)

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
  const kickEnvelope = updateTerrainKickEnvelope(time, props.playing, props.reducedMotion)
  const activeRipples = activeTerrainRipples(time, props.reducedMotion, waveTuning)
  const centerPulse = activeTerrainCenterPulse(time, props.reducedMotion, waveTuning)

  terrainUniforms.uOpacity.value = terrainLayerOpacity(preset)
  terrainUniforms.uTime.value = time
  terrainUniforms.uFogColor.value.set(terrainPalette.fog)
  terrainUniforms.uBass.value = bass
  terrainUniforms.uMid.value = mid
  terrainUniforms.uTreble.value = treble
  terrainUniforms.uBeat.value = beat
  terrainUniforms.uVolume.value = volume
  terrainUniforms.uSubBass.value = subBass
  terrainUniforms.uLowMid.value = lowMid
  terrainUniforms.uHighMid.value = highMid
  terrainUniforms.uEnergy.value = energy
  terrainUniforms.uTriggerEnergy.value = triggerEnergy
  terrainUniforms.uKickEnvelope.value = kickEnvelope
  terrainUniforms.uCenterPulseData.value.set(
    centerPulse.radius,
    Math.max(0.001, centerPulse.width),
    centerPulse.fade,
    centerPulse.strength,
  )
  terrainUniforms.uIntensity.value = intensity
  terrainUniforms.uReducedMotion.value = props.reducedMotion ? 1 : 0
  terrainUniforms.uMaxHeight.value = maxHeight
  terrainUniforms.uBaseHeight.value = 0.064
  terrainUniforms.uResponse.value = immediate ? 1 : responseTuning
  terrainUniforms.uWave.value = waveTuning
  terrainUniforms.uColorLow.value.set(terrainPalette.coolCore)
  terrainUniforms.uColorMid.value.set(terrainPalette.coolEdge)
  terrainUniforms.uColorHigh.value.set(terrainPalette.rim)
  terrainUniforms.uColorPulse.value.set(terrainPalette.warmCore)
  terrainUniforms.uColorShadow.value.set(terrainPalette.fog)
  terrainUniforms.uBaseColor1.value.set(terrainPalette.baseDeep)
  terrainUniforms.uBaseColor2.value.set(terrainPalette.baseLift)
  terrainUniforms.uCoolCore.value.set(terrainPalette.coolCore)
  terrainUniforms.uCoolEdge.value.set(terrainPalette.coolEdge)
  terrainUniforms.uWarmCore.value.set(terrainPalette.warmCore)
  terrainUniforms.uWarmEdge.value.set(terrainPalette.warmEdge)
  terrainUniforms.uRippleColor.value.set(terrainPalette.ripple)
  terrainUniforms.uSparkColor.value.set(terrainPalette.spark)
  terrainUniforms.uRimColor.value.set(terrainPalette.rim)
  terrainUniforms.uGlowIntensity.value = terrainPalette.glowIntensity
  terrainUniforms.uWarmth.value = warmth
  terrainUniforms.uBrightness.value = brightness
  terrainUniforms.uPresence.value = presence
  terrainUniforms.uBrilliance.value = brilliance
  terrainUniforms.uAir.value = air
  terrainUniforms.uSharpness.value = sharpness
  updateTerrainRippleUniforms(terrainUniforms, activeRipples)
}

function updateTerrainRippleUniforms(uniforms: TerrainUniforms, ripples: ActiveTerrainRipple[]) {
  const count = Math.min(ripples.length, TERRAIN_RIPPLE_LIMIT)
  uniforms.uRippleCount.value = count

  for (let index = 0; index < TERRAIN_RIPPLE_LIMIT; index += 1) {
    const ripple = ripples[index]
    const pos = uniforms.uRipplePos.value[index]
    const data = uniforms.uRippleData.value[index]
    if (ripple) {
      pos.set(ripple.x, ripple.z)
      data.set(ripple.radius, Math.max(0.001, ripple.width), ripple.fade, ripple.strength)
      uniforms.uRippleKind.value[index] = terrainRippleKindCode(ripple.kind)
    } else {
      pos.set(999, 999)
      data.set(0, 1, 0, 0)
      uniforms.uRippleKind.value[index] = 0
    }
  }
}

function terrainRippleKindCode(kind: TerrainRippleKind) {
  if (kind === 'spark') {
    return 2
  }
  if (kind === 'snare') {
    return 1
  }
  return 0
}

function terrainGridSize(preset: MusicVisualStagePreset, reducedMotion: boolean, densityTuning = 1) {
  if (preset === 'dj') {
    const baseSize = reducedMotion ? 120 : 176
    const minSize = reducedMotion ? 54 : 82
    const maxSize = reducedMotion ? 220 : 300
    return Math.round(clamp(baseSize * densityTuning, minSize, maxSize))
  }
  return reducedMotion ? 34 : 42
}

function terrainMaxHeight(preset: MusicVisualStagePreset) {
  if (preset === 'dj') {
    return 1.34
  }
  return 0.95
}

function terrainLayerOpacity(preset: MusicVisualStagePreset) {
  if (preset === 'galaxy') {
    return 0
  }
  return 1
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
  terrainInfoAttribute = null
  terrainBandAttribute = null
  terrainCells = []
  resetTerrainRipples()
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
  const cameraBoost = isDjPreset ? 0.58 : 1
  const driftScale = props.stagePreset === 'galaxy' ? 1.36 : 0.16
  const radiusBias = props.stagePreset === 'galaxy' ? 0.62 : 2.05
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
      beatPunch * 0.42 * motionScale * cameraBoost -
      clamp(props.intensity, 0.2, 1) * 0.22

  orbitYaw += (targetYaw - orbitYaw) * (props.stageDragging ? 0.22 : 0.105)
  orbitPitch += (targetPitch - orbitPitch) * 0.1
  orbitRadius += (targetRadius - orbitRadius) * 0.075
  cameraPunch = Math.max(cameraPunch * 0.86, beatPunch * 0.72 * motionScale * cameraBoost)

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
  camera.rotation.z += Math.sin(time * 0.9) * cameraPunch * 0.008 * cameraBoost
  const targetFov = isDjPreset ? 46 - cameraPunch * 0.62 : 46 - cameraPunch * 2.6 * cameraBoost
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
  const presetSpin = props.stagePreset === 'galaxy' ? 1.72 : 0.18
  const presetLift = props.stagePreset === 'dj' ? 0.26 : 1
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
precision highp float;

attribute vec4 aTerrainInfo;
attribute vec4 aTerrainBand;

uniform float uTime;
uniform float uBass;
uniform float uMid;
uniform float uTreble;
uniform float uBeat;
uniform float uVolume;
uniform float uSubBass;
uniform float uLowMid;
uniform float uHighMid;
uniform float uEnergy;
uniform float uTriggerEnergy;
uniform float uKickEnvelope;
uniform float uIntensity;
uniform float uReducedMotion;
uniform float uMaxHeight;
uniform float uBaseHeight;
uniform float uResponse;
uniform float uWave;
uniform vec3 uColorLow;
uniform vec3 uColorMid;
uniform vec3 uColorHigh;
uniform vec3 uColorPulse;
uniform vec3 uColorShadow;
uniform vec3 uFogColor;
uniform vec3 uBaseColor1;
uniform vec3 uBaseColor2;
uniform vec3 uCoolCore;
uniform vec3 uCoolEdge;
uniform vec3 uWarmCore;
uniform vec3 uWarmEdge;
uniform vec3 uRippleColor;
uniform vec3 uSparkColor;
uniform vec3 uRimColor;
uniform float uGlowIntensity;
uniform float uWarmth;
uniform float uBrightness;
uniform float uPresence;
uniform float uBrilliance;
uniform float uAir;
uniform float uSharpness;
uniform float uRippleCount;
uniform vec2 uRipplePos[${TERRAIN_RIPPLE_LIMIT}];
uniform vec4 uRippleData[${TERRAIN_RIPPLE_LIMIT}];
uniform float uRippleKind[${TERRAIN_RIPPLE_LIMIT}];
uniform vec4 uCenterPulseData;

varying vec3 vTerrainColor;
varying vec3 vTerrainLocalPosition;
varying vec3 vTerrainNormal;
varying float vTerrainTop;
varying float vTerrainDepth;
varying float vTerrainFog;
varying float vTerrainHeightRatio;
varying float vTerrainRippleGlow;
varying float vTerrainRippleSharp;
varying float vTerrainLiftAccent;
varying float vTerrainImpactAccent;
varying float vTerrainRing;
varying float vTerrainSeed;
varying float vTerrainEdgeFade;
varying float vTerrainDistance;
varying float vTerrainBand;
varying float vTerrainWarmZone;

const float PI = 3.141592653589793;

void main() {
  float ring = aTerrainInfo.x;
  float angle = aTerrainInfo.y;
  float spiral = aTerrainInfo.z;
  float seed = aTerrainInfo.w;
  float band = aTerrainBand.x;
  float lane = aTerrainBand.y;
  float edgeFade = aTerrainBand.z;
  float yPos = position.y + 0.5;
  float motion = mix(1.0, 0.38, uReducedMotion);
  float time = uTime * (0.72 + clamp(uResponse, 0.2, 2.4) * 0.38) * motion;
  vec4 instanceOrigin = instanceMatrix * vec4(0.0, 0.0, 0.0, 1.0);
  vec2 cellPos = instanceOrigin.xz;

  float rippleLift = 0.0;
  float rippleGlow = 0.0;
  float rippleSharp = 0.0;
  for (int i = 0; i < ${TERRAIN_RIPPLE_LIMIT}; i++) {
    if (float(i) < uRippleCount) {
      vec4 rippleData = uRippleData[i];
      float distanceToWave = length(cellPos - uRipplePos[i]);
      float delta = distanceToWave - rippleData.x;
      float wave = exp(-(delta * delta) / max(0.001, rippleData.y)) * rippleData.z * rippleData.w;
      float localFade = 1.0 - edgeFade * 0.18;
      float kind = uRippleKind[i];
      if (kind > 1.5) {
        rippleSharp += wave * 0.9 * localFade;
        rippleGlow += wave * 0.62 * localFade;
        rippleLift += wave * 0.16 * localFade;
      } else if (kind > 0.5) {
        rippleSharp += wave * 0.72 * localFade;
        rippleGlow += wave * 0.76 * localFade;
        rippleLift += wave * 0.2 * localFade;
      } else {
        rippleGlow += wave * 0.94 * localFade;
        rippleLift += wave * 0.42 * localFade;
      }
    }
  }

  if (uCenterPulseData.w > 0.0) {
    float centerDistance = length(cellPos);
    float centerDelta = centerDistance - uCenterPulseData.x;
    float centerWave = exp(-(centerDelta * centerDelta) / max(0.001, uCenterPulseData.y)) * uCenterPulseData.z * uCenterPulseData.w;
    float centerFillRamp = smoothstep(0.18, 1.15, uCenterPulseData.x);
    float centerFillEdge = 1.0 - smoothstep(
      max(0.0, uCenterPulseData.x - uCenterPulseData.y * 5.2),
      uCenterPulseData.x + uCenterPulseData.y * 1.25,
      centerDistance
    );
    float centerFill = centerFillEdge * centerFillRamp * uCenterPulseData.z * uCenterPulseData.w;
    float centerLocalFade = 1.0 - edgeFade * 0.18;
    rippleGlow += centerWave * 0.94 * centerLocalFade;
    rippleLift += centerWave * 0.42 * centerLocalFade;
    rippleGlow += centerFill * 0.28 * centerLocalFade;
    rippleLift += centerFill * (0.18 + centerFillEdge * 0.04) * centerLocalFade;
  }

  float visibleFade = 1.0 - edgeFade * 0.34;
  float innerFade = 0.5 + smoothstep(0.02, 0.2, ring) * 0.5;
  float radialGate = innerFade * visibleFade;
  float centerCoreRaw = 1.0 - smoothstep(0.02, 0.2, ring);
  float centerCore = centerCoreRaw;
  float innerBand = 1.0 - smoothstep(0.12, 0.44, ring);
  float transientInnerBand = innerBand;
  float middleBand = 0.0;
  float outerBand = smoothstep(0.48, 0.86, ring) * (1.0 - edgeFade * 0.28);
  float localWave = clamp(rippleLift * uWave, 0.0, 1.0);
  float localRippleGlow = clamp(rippleGlow * (0.72 + uWave * 0.28), 0.0, 1.0);
  float expansionRippleSharp = rippleSharp;
  float rippleTailPattern = (sin(ring * 32.0 + spiral * 2.4 - time * 0.34 + localRippleGlow * 2.1) + 1.0) * 0.5;
  float rippleTail = rippleTailPattern;
  float radialPhase =
    ring * 18.0 +
    sin(angle) * 1.45 +
    cos(angle) * 0.9 -
    time * 0.52 +
    seed * 1.4;
  float lateralSlowFlow = clamp(
    (
      sin(cellPos.x * 0.34 + cellPos.y * 0.21 - time * 0.22 + seed * 2.6) +
      cos(cellPos.x * 0.18 - cellPos.y * 0.3 + time * 0.16 + seed * 1.9)
    ) * 0.18 + 0.5,
    0.0,
    1.0
  );
  float slowFlow = lateralSlowFlow;
  float radialCurrent = max(0.0, sin(radialPhase));
  float spikeGate = seed > 0.925 ? max(0.0, sin(time * 5.8 + seed * PI * 2.0)) : 0.0;
  float microSpark = seed > 0.982 ? max(0.0, sin(time * 8.4 + seed * PI * 4.0)) : 0.0;
  float kickImpact = clamp(uKickEnvelope, 0.0, 1.0);
  float kickLift = kickImpact * (centerCore * 0.54 + transientInnerBand * 0.16) * (0.9 + uWave * 0.08);
  float coreLift = centerCore * (uSubBass * 0.34 + uBass * 0.16 + uBeat * 0.1 + uVolume * 0.055);
  float bassChunkLift = uBass * (0.1 + transientInnerBand * 0.24 + middleBand * 0.14) * (0.58 + slowFlow * 0.42);
  float waveLift = localWave * (0.58 + rippleTail * 0.2) * (1.0 + transientInnerBand * 0.26 + middleBand * 0.16);
  float lowMidLift = uLowMid * slowFlow * (0.13 + middleBand * 0.21);
  float midLift = uMid * radialCurrent * (0.12 + middleBand * 0.22 + outerBand * 0.08);
  float highMidLift = uHighMid * (spikeGate * outerBand * 0.12 + expansionRippleSharp * 0.22);
  float energySpike = microSpark * uBeat * uEnergy * (0.07 + outerBand * 0.08);
  float terrainEnergy = clamp(
    (coreLift + bassChunkLift + kickLift + lowMidLift + midLift + highMidLift + energySpike) * radialGate - 0.03,
    0.0,
    1.25
  );
  float rippleEnergy = clamp(waveLift * radialGate, 0.0, 1.18);
  float rippleHeight = pow(rippleEnergy, 0.82) * uMaxHeight * (0.24 + uWave * 0.08);
  float kickHeight = kickImpact * (centerCore * 0.18 + transientInnerBand * 0.05) * uMaxHeight * (0.32 + uWave * 0.04);
  float ringRipple =
    (sin(ring * 34.0 - time * 0.32 + spiral * 3.1) * 0.005 + (slowFlow - 0.5) * 0.007) *
    radialGate;
  float idleRelief = (0.018 + seed * 0.012 + slowFlow * 0.012) * radialGate * (1.0 - edgeFade * 0.4);
  float horizonBase = uBaseHeight * (0.3 + radialGate * 0.7);
  float targetHeight =
    horizonBase +
    idleRelief +
    pow(terrainEnergy, 0.86) * uMaxHeight +
    kickHeight +
    rippleHeight +
    ringRipple;
  float height = clamp(targetHeight, 0.006, uMaxHeight * 1.18 + 0.1);
  float heightRatio = clamp(height / max(0.1, uMaxHeight), 0.0, 1.0);

  vec3 pos = position;
  pos.y = -0.5 + yPos * height;

  float waveGlow = clamp(
    localRippleGlow * (0.72 + uTriggerEnergy * 0.24) +
      localWave * 0.1 +
      centerCore * uSubBass * 0.1 +
      centerCore * kickImpact * 0.28 +
      spikeGate * uHighMid * 0.12,
    0.0,
    1.0
  );
  float amplifiedRippleSharp = clamp(
    expansionRippleSharp * 0.72 + kickImpact * (centerCore * 0.42 + transientInnerBand * 0.12) + energySpike * 1.4 + spikeGate * uHighMid * 0.28,
    0.0,
    1.0
  );
  float liftGlow = smoothstep(0.08, 0.78, heightRatio);
  float liftAccent = clamp(
    liftGlow * (0.22 + uBass * 0.16 + uBeat * 0.18) +
      waveGlow * 0.22 +
      localWave * (0.08 + uTriggerEnergy * 0.08),
    0.0,
    1.0
  );
  float impactAccent = clamp(
    smoothstep(0.18, 0.78, amplifiedRippleSharp) * (0.56 + uBeat * 0.18) +
      kickImpact * (centerCore * 0.24 + transientInnerBand * 0.14),
    0.0,
    1.0
  );
  vec3 color = uColorLow;
  color = mix(color, uColorMid, clamp(ring * 0.42 + slowFlow * 0.18 + uLowMid * 0.08, 0.0, 1.0));
  color = mix(color, uColorHigh, clamp(waveGlow * 0.28 + heightRatio * 0.18 + liftAccent * 0.1 + highMidLift * 0.3, 0.0, 1.0));
  color = mix(
    color,
    uColorPulse,
    clamp(waveGlow * uBeat * 0.13 + localWave * (0.1 + uTriggerEnergy * 0.1) + expansionRippleSharp * 0.14 + energySpike * 0.22 + impactAccent * 0.14, 0.0, 1.0)
  );
  color = mix(color, uColorShadow, clamp(edgeFade * 0.38, 0.0, 1.0));
  color *= clamp(0.38 + radialGate * 0.28 + heightRatio * 0.23 + liftAccent * 0.18 + waveGlow * 0.24 + impactAccent * 0.12 + uEnergy * 0.045, 0.0, 1.22);

  vTerrainColor = clamp(color, vec3(0.0), vec3(0.86));
  vTerrainLocalPosition = position;
  vTerrainNormal = normal;
  vTerrainTop = yPos;
  vTerrainHeightRatio = heightRatio;
  vTerrainRippleGlow = waveGlow;
  vTerrainRippleSharp = amplifiedRippleSharp;
  vTerrainLiftAccent = liftAccent;
  vTerrainImpactAccent = impactAccent;
  vTerrainRing = ring;
  vTerrainSeed = seed;
  vTerrainEdgeFade = edgeFade;
  vTerrainDistance = length(cellPos);
  vTerrainBand = band;
  vTerrainWarmZone = clamp(
      centerCore * (0.34 + kickImpact * 0.18) +
      transientInnerBand * 0.2 +
      middleBand * 0.22 +
      slowFlow * 0.16 +
      lane * 0.08,
    0.0,
    1.0
  );

  vec4 mvPosition = modelViewMatrix * instanceMatrix * vec4(pos, 1.0);
  float depthFog = smoothstep(7.6, 15.8, -mvPosition.z);
  vTerrainFog = clamp(edgeFade * 0.4 + depthFog * 0.6, 0.0, 1.0);
  vTerrainDepth = clamp(1.0 - depthFog * 0.36, 0.5, 1.0);
  gl_Position = projectionMatrix * mvPosition;
}
`

const terrainFragmentShaderSource = `
precision highp float;

uniform float uOpacity;
uniform vec3 uFogColor;
uniform vec3 uBaseColor1;
uniform vec3 uBaseColor2;
uniform vec3 uCoolCore;
uniform vec3 uCoolEdge;
uniform vec3 uWarmCore;
uniform vec3 uWarmEdge;
uniform vec3 uRippleColor;
uniform vec3 uSparkColor;
uniform vec3 uRimColor;
uniform float uGlowIntensity;
uniform float uWarmth;
uniform float uBrightness;
uniform float uPresence;
uniform float uBrilliance;
uniform float uAir;
uniform float uSharpness;
uniform float uEnergy;
uniform float uBeat;

varying vec3 vTerrainColor;
varying vec3 vTerrainLocalPosition;
varying vec3 vTerrainNormal;
varying float vTerrainTop;
varying float vTerrainDepth;
varying float vTerrainFog;
varying float vTerrainHeightRatio;
varying float vTerrainRippleGlow;
varying float vTerrainRippleSharp;
varying float vTerrainLiftAccent;
varying float vTerrainImpactAccent;
varying float vTerrainRing;
varying float vTerrainSeed;
varying float vTerrainEdgeFade;
varying float vTerrainDistance;
varying float vTerrainBand;
varying float vTerrainWarmZone;

float terrainRandom(float value) {
  return fract(sin(value * 127.1 + 17.17) * 43758.5453123);
}

void main() {
  float topFace = smoothstep(0.48, 0.86, vTerrainNormal.y) * smoothstep(0.62, 0.98, vTerrainTop);
  float vertical = clamp(vTerrainTop, 0.0, 1.0);
  float sideEdge = max(
    smoothstep(0.34, 0.5, abs(vTerrainLocalPosition.x)),
    smoothstep(0.34, 0.5, abs(vTerrainLocalPosition.z))
  );
  float topEdge = sideEdge * topFace;
  float seedA = terrainRandom(vTerrainSeed + vTerrainBand * 2.7);
  float seedB = terrainRandom(vTerrainSeed * 3.1 + vTerrainRing * 5.3);
  float localSpark = terrainRandom(vTerrainSeed * 11.7 + floor(uBeat * 8.0) + floor(uEnergy * 6.0));
  float heightGlow = smoothstep(0.015, 0.48, vTerrainHeightRatio);
  float liftAccent = clamp(vTerrainLiftAccent, 0.0, 1.0);
  float impactAccent = clamp(vTerrainImpactAccent, 0.0, 1.0);
  float distanceFade = 1.0 - smoothstep(0.6, 1.0, vTerrainRing);
  float surfaceFade = 1.0 - vTerrainEdgeFade * 0.36;

  float warmBlend = clamp(
    uWarmth * 0.64 +
      vTerrainWarmZone * 0.22 +
      liftAccent * 0.08 +
      impactAccent * 0.1 +
      (1.0 - vTerrainRing) * 0.14 -
      uBrightness * 0.03,
    0.0,
    1.0
  );
  vec3 coolZone = mix(uCoolCore, uCoolEdge, clamp(seedA * 0.72 + vTerrainBand * 0.28, 0.0, 1.0));
  vec3 warmZone = mix(uWarmCore, uWarmEdge, clamp(seedB * 0.62 + (1.0 - vTerrainRing) * 0.38, 0.0, 1.0));
  vec3 glowColor = mix(coolZone, warmZone, warmBlend);
  glowColor = mix(glowColor, uSparkColor, clamp(uBrightness * 0.16 + vTerrainRippleSharp * 0.1 + impactAccent * 0.16, 0.0, 0.44));

  float materialGlow = clamp(
    heightGlow * (0.46 + vTerrainHeightRatio * 0.34) +
      vTerrainRippleGlow * 0.36 +
      vTerrainRippleSharp * 0.26 +
      liftAccent * 0.34 +
      impactAccent * 0.3 +
      uEnergy * 0.08,
    0.0,
    1.65
  ) * uGlowIntensity * (0.72 + distanceFade * 0.28) * surfaceFade;

  float intrinsicLift = clamp(
    smoothstep(0.05, 0.82, vTerrainHeightRatio) * 0.62 +
      liftAccent * 0.62 +
      impactAccent * 0.34,
    0.0,
    1.48
  );
  float bodyBrightness = clamp(0.72 + intrinsicLift * 0.46 + uBrightness * 0.08 + impactAccent * 0.12, 0.58, 1.48);
  vec3 bodyBaseColor = mix(uBaseColor1, uBaseColor2, vertical * 0.5 + vTerrainHeightRatio * 0.16 + distanceFade * 0.06);
  vec3 intrinsicColor = mix(vTerrainColor, glowColor, clamp(vTerrainRippleGlow * 0.08 + impactAccent * 0.06, 0.0, 0.22));
  vec3 bodyColor = mix(
    bodyBaseColor,
    intrinsicColor,
    clamp(0.26 + intrinsicLift * 0.44 + vTerrainHeightRatio * 0.12, 0.0, 0.88)
  ) * bodyBrightness;
  float sharpSideFalloff = mix(0.52, 0.18, clamp(uSharpness, 0.0, 1.0));
  float sideGlow = (1.0 - smoothstep(0.02, sharpSideFalloff, 1.0 - vertical)) * heightGlow;
  vec3 sideColor = mix(bodyColor, glowColor, sideGlow * (0.44 + materialGlow * 0.58 + liftAccent * 0.18));
  sideColor += intrinsicColor * (0.035 + intrinsicLift * 0.12) * (0.55 + vertical * 0.45);
  sideColor += glowColor * sideEdge * (0.04 + vTerrainHeightRatio * 0.07 + vTerrainRippleGlow * 0.1 + liftAccent * 0.12 + impactAccent * 0.08);

  float idleTopTone = (0.045 + distanceFade * 0.04) * (1.0 - vTerrainEdgeFade * 0.4);
  float topIntensity = clamp(idleTopTone + materialGlow * 0.84 + vTerrainHeightRatio * 0.24 + vTerrainRippleGlow * 0.26 + liftAccent * 0.24 + impactAccent * 0.18, 0.0, 1.55);
  vec3 topBaseColor = mix(uBaseColor2, intrinsicColor, clamp(0.3 + intrinsicLift * 0.38, 0.0, 0.82));
  vec3 topColor = mix(topBaseColor, glowColor, topIntensity);
  topColor = mix(topColor, uWarmCore, clamp(liftAccent * 0.14 + impactAccent * 0.2, 0.0, 0.38));
  topColor += uRimColor * topEdge * (0.12 + vTerrainHeightRatio * 0.28 + uBrilliance * 0.16);
  topColor += uRippleColor * vTerrainRippleGlow * (0.38 + uBeat * 0.22 + liftAccent * 0.1);
  topColor += uSparkColor * impactAccent * (0.08 + topFace * 0.12);

  float idleAirSpark = step(0.955, seedA) * smoothstep(0.0, 0.32, uAir) * (1.0 - smoothstep(0.08, 0.3, vTerrainHeightRatio));
  float presenceFlash = step(0.978 - uPresence * 0.09, localSpark) *
    (0.5 + 0.5 * sin(vTerrainSeed * 45.0 + uBeat * 12.0 + uEnergy * 8.0));
  float brillianceFlash = topEdge * step(0.986 - uBrilliance * 0.1, seedB) * (0.36 + uBrilliance * 0.74);
  float spark = clamp(idleAirSpark * 0.16 + presenceFlash * uPresence * 0.22 + brillianceFlash * 0.18 + vTerrainRippleSharp * 0.18 + impactAccent * 0.22, 0.0, 0.72);
  topColor += uSparkColor * spark;

  vec3 color = mix(sideColor, topColor, topFace);
  color *= 0.6 + vertical * 0.32 + topFace * 0.14 + liftAccent * 0.06 + impactAccent * 0.04;
  color += intrinsicColor * intrinsicLift * (0.12 + topFace * 0.1 + impactAccent * 0.06) * (1.0 - vTerrainFog * 0.32);
  color += uRimColor * topEdge * (0.025 + uAir * 0.035) * (1.0 - vTerrainFog);
  color = mix(color, uRippleColor, clamp(vTerrainRippleGlow * 0.12 + vTerrainRippleSharp * 0.07 + impactAccent * 0.08, 0.0, 0.24));

  float farSilhouette = smoothstep(0.46, 0.95, vTerrainFog);
  vec3 atmosphericColor = mix(uBaseColor1, uBaseColor2, 0.35);
  color = mix(color, atmosphericColor, farSilhouette * 0.36);
  color = mix(color, uFogColor * 0.72, vTerrainFog * 0.3);
  color = clamp(color * vTerrainDepth * uOpacity, vec3(0.0), vec3(1.12));

  gl_FragColor = vec4(color, 1.0);
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
  float galaxyPreset = presetMatch(1.0);
  float djPreset = presetMatch(3.0);
  float rawTime = uTime * motion;
  float rhythmTime = uRhythmTime * motion;
  float t = mix(rawTime, rhythmTime, galaxyPreset);
  float energy = bandEnergy(aBand);
  float twinkle = pow(0.5 + 0.5 * sin(t * (0.65 + aSeed * 1.25) + aSeed * TAU), 4.0);
  float stagePower = 1.0 + djPreset * 0.46;
  float driftPower = 1.0 + galaxyPreset * 0.72 + djPreset * 0.18;
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
    } else if (djPreset > 0.5) {
      float scanRow = floor(aLane * 14.0) / 13.0;
      pos.x = (aBand - 0.5) * 8.4;
      pos.y = -1.06 + scanRow * 0.22 + sin(t * 1.6 + aBand * TAU) * 0.04;
      pos.z = -4.05 + scanRow * 4.65 + fract(aSeed + t * 0.08) * 0.18;
      color = mix(uColorLine, uColorPulse, twinkle * 0.45 + uTreble * 0.22);
      glow = twinkle * 0.24 + uBeat * 0.26;
      alpha = 0.04 + twinkle * 0.11 + uVolume * 0.06;
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
    } else if (djPreset > 0.5) {
      float gridRow = floor(aLane * 10.0) / 9.0;
      pos.x = fract(aBand + t * 0.06 + aSeed * 0.18) * 8.4 - 4.2;
      pos.y = -1.05 + gridRow * 0.24 + sin(t * 1.35 + aSeed * TAU) * 0.05;
      pos.z = -4.1 + gridRow * 4.65;
      color = mix(uColorLine, uColorPulse, ribbonEnergy * 0.56 + twinkle * 0.2);
      glow = ribbonEnergy * 0.46 + twinkle * 0.18;
      alpha = 0.045 + ribbonEnergy * 0.18 + twinkle * 0.07;
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
    float bandPulse = smoothstep(0.04, 1.0, columnEnergy + uBeat * (0.25 + djPreset * 0.18));
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
      vec3 stageLow = uColorLow;
      vec3 stageMid = uColorMid;
      vec3 stageHigh = uColorHigh;
      stageMid = mix(stageMid, uColorHalo, galaxyPreset * 0.42);
      color = mix(stageLow, stageMid, aBand);
      color = mix(color, stageHigh, bandPulse * 0.52);
      glow = bandPulse + twinkle * 0.18;
      alpha = 0.10 + bandPulse * 0.48 + twinkle * 0.08;
      alpha *= 1.0 - galaxyPreset * 0.18;
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
    } else if (djPreset > 0.5) {
      radius = 0.7 + ring * 2.2 + ringEnergy * 0.9;
      pos.y = -1.02 + floor(aLane * 8.0) * 0.055 + ringEnergy * 0.22;
      pos.z = -3.65 + ring * 3.8;
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
    vec3 burstBase = mix(uColorRipple, uColorLine, djPreset * 0.52);
    vec3 burstPeak = uColorHalo;
    burstPeak = mix(burstPeak, uColorPulse, djPreset * 0.4);
    color = mix(burstBase, burstPeak, ringEnergy * 0.48 + ring * 0.2);
    glow = ringEnergy + twinkle * 0.16;
    alpha = 0.07 + ringEnergy * 0.42 + (1.0 - ring) * 0.08;
    alpha *= 1.0 - galaxyPreset * 0.22;
  } else {
    float linePulse = max(uBeat * 0.75, energy * 0.45);
    float path = aLane * TAU * 2.0 + t * 0.35;
    pos.xz = rotate2d(t * 0.025 * driftPower + uBass * 0.08) * pos.xz;
    pos.y += sin(path + aSeed * 5.0) * (0.12 + uMid * 0.28);

    if (djPreset > 0.5) {
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
    } else {
      pos.xyz *= 0.82 + linePulse * 0.04;
      pos.y *= 0.62;
      pos.z -= 0.18;
    }

    vec3 trailBase = mix(uColorLine, uColorShadow, djPreset * 0.28);
    vec3 trailPeak = mix(uColorPulse, uColorHigh, galaxyPreset * 0.28);
    color = mix(trailBase, trailPeak, 0.42 + linePulse * 0.48);
    glow = linePulse + twinkle * 0.2;
    alpha = 0.06 + linePulse * 0.34 + twinkle * 0.12;
    alpha *= 1.0 + galaxyPreset * 0.1 + djPreset * 0.16;
  }

  float layerBoost = uLayerScale > 2.0 ? 0.55 : 1.0;
  vColor = color;
  vGlow = glow;
  float presetAlpha = 1.0 + galaxyPreset * 0.08;
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

function terrainMaterialPalette(theme: MusicImmersiveTheme | string, mode: MusicVisualMode): TerrainMaterialPalette {
  if (theme === 'animal-island') {
    return {
      baseDeep: '#1c2116',
      baseLift: '#3f3b24',
      fog: '#1a1710',
      coolCore: '#52d6b5',
      coolEdge: '#7fb6a6',
      warmCore: '#ffe0a1',
      warmEdge: '#b48244',
      ripple: '#92e5bd',
      spark: '#fff8e8',
      rim: '#ffe9b9',
      glowIntensity: 0.98,
    }
  }
  if (theme === 'cinema') {
    return {
      baseDeep: '#130810',
      baseLift: '#2a141f',
      fog: '#16070d',
      coolCore: '#7ad7c2',
      coolEdge: '#326c73',
      warmCore: '#ff6b72',
      warmEdge: '#f3b86f',
      ripple: '#ff8c92',
      spark: '#fff4d6',
      rim: '#ffd89a',
      glowIntensity: 1.02,
    }
  }
  if (theme === 'galaxy') {
    return {
      baseDeep: '#061225',
      baseLift: '#132b4b',
      fog: '#07142d',
      coolCore: '#73a7ff',
      coolEdge: '#7e5cff',
      warmCore: '#fff0b8',
      warmEdge: '#9cffdf',
      ripple: '#6d83ff',
      spark: '#f9fcff',
      rim: '#cce8ff',
      glowIntensity: 1.16,
    }
  }
  if (theme === 'neon') {
    return {
      baseDeep: '#0c0718',
      baseLift: '#231044',
      fog: '#10061f',
      coolCore: '#00f5d4',
      coolEdge: '#7df9ff',
      warmCore: '#ff4fd8',
      warmEdge: '#b76cff',
      ripple: '#00f5d4',
      spark: '#eff7ff',
      rim: '#f6c8ff',
      glowIntensity: 1.24,
    }
  }
  if (theme === 'sunset') {
    return {
      baseDeep: '#211018',
      baseLift: '#4a2231',
      fog: '#281018',
      coolCore: '#f4d28a',
      coolEdge: '#ff9b72',
      warmCore: '#ff705f',
      warmEdge: '#ffb861',
      ripple: '#ff9b72',
      spark: '#fff8df',
      rim: '#ffe4aa',
      glowIntensity: 1.08,
    }
  }
  if (theme === 'midnight') {
    return {
      baseDeep: '#07101d',
      baseLift: '#16253b',
      fog: '#07101d',
      coolCore: '#9fb7d9',
      coolEdge: '#5f7da9',
      warmCore: '#dceaff',
      warmEdge: '#7898cb',
      ripple: '#7898cb',
      spark: '#eef6ff',
      rim: '#c7d8ee',
      glowIntensity: 0.86,
    }
  }
  if (mode === 'sleep') {
    return {
      baseDeep: '#071322',
      baseLift: '#172a44',
      fog: '#081526',
      coolCore: '#86b7ff',
      coolEdge: '#4d6f9f',
      warmCore: '#dceaff',
      warmEdge: '#8fb3ff',
      ripple: '#86b7ff',
      spark: '#ffffff',
      rim: '#cfe1ff',
      glowIntensity: 0.82,
    }
  }
  if (mode === 'focus') {
    return {
      baseDeep: '#0d211d',
      baseLift: '#1f4038',
      fog: '#0d211d',
      coolCore: '#78d3c7',
      coolEdge: '#4aa798',
      warmCore: '#b6e6cf',
      warmEdge: '#6fd8c7',
      ripple: '#6fd8c7',
      spark: '#f5fff9',
      rim: '#c8f6e3',
      glowIntensity: 0.92,
    }
  }
  return {
    baseDeep: '#07111f',
    baseLift: '#10253a',
    fog: '#0b1624',
    coolCore: '#35d4ff',
    coolEdge: '#2a8ed9',
    warmCore: '#ffca72',
    warmEdge: '#ff8a5c',
    ripple: '#77d6ff',
    spark: '#f8fbff',
    rim: '#d7faff',
    glowIntensity: 1.08,
  }
}

function stagePresetCode(preset: MusicVisualStagePreset) {
  const codes: Record<MusicVisualStagePreset, number> = {
    galaxy: 1,
    dj: 3,
  }
  return codes[preset] ?? codes.dj
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
  terrainKickEnvelope = 0
  terrainKickTarget = 0
  lastTerrainKickEnvelopeTime = 0
  terrainCenterPulseStartedAt = -999
  terrainCenterPulseStrength = 0
  terrainCenterPulseSpeedScale = 1
}

function inactiveTerrainCenterPulse() {
  return { radius: 0, width: TERRAIN_PULSE_WIDTH, fade: 0, strength: 0 }
}

function clearTerrainCenterPulse() {
  terrainCenterPulseStartedAt = -999
  terrainCenterPulseStrength = 0
  terrainCenterPulseSpeedScale = 1
}

function triggerTerrainKickEnvelope(strength: number) {
  const motionScale = props.reducedMotion ? 0.62 : 1
  const impulse = clamp(Math.max(TERRAIN_KICK_MIN_IMPULSE, strength * 0.88) * motionScale, 0, 1)
  terrainKickTarget = clamp(Math.max(terrainKickTarget, impulse), 0, 1)
  terrainKickEnvelope = clamp(Math.max(terrainKickEnvelope, impulse * 0.68), 0, 1)
}

function triggerTerrainCenterPulse(time: number, strength: number) {
  const safeStrength = clamp(strength, 0, 1.08)
  if (safeStrength <= 0.08) {
    return
  }

  terrainCenterPulseStartedAt = time
  terrainCenterPulseStrength = safeStrength
  terrainCenterPulseSpeedScale = terrainRippleSpeedScale('pulse', safeStrength)
  triggerTerrainKickEnvelope(safeStrength)
}

function updateTerrainKickEnvelope(time: number, playing: boolean, reducedMotion: boolean) {
  const delta =
    lastTerrainKickEnvelopeTime > 0
      ? clamp(time - lastTerrainKickEnvelopeTime, 0, 0.08)
      : 1 / 60
  lastTerrainKickEnvelopeTime = time

  const targetDecayRate = reducedMotion ? TERRAIN_KICK_TARGET_DECAY_RATE * 1.35 : TERRAIN_KICK_TARGET_DECAY_RATE
  const responseRate = reducedMotion ? TERRAIN_KICK_RESPONSE_RATE * 0.72 : TERRAIN_KICK_RESPONSE_RATE
  const releaseBoost = playing ? 1 : 1.8
  const targetDecay = 1 - Math.exp(-delta * targetDecayRate * releaseBoost)
  terrainKickTarget += (0 - terrainKickTarget) * clamp(targetDecay, 0, 1)

  const response = 1 - Math.exp(-delta * responseRate * releaseBoost)
  terrainKickEnvelope += (terrainKickTarget - terrainKickEnvelope) * clamp(response, 0, 1)

  if (terrainKickEnvelope < 0.002 && terrainKickTarget < 0.002) {
    terrainKickEnvelope = 0
    terrainKickTarget = 0
  }

  return clamp01(terrainKickEnvelope)
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
        if (trigger.kind === 'pulse') {
          triggerTerrainCenterPulse(time, strength)
        }
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
    const centerStrength = clamp01(bassPush * strengthScale * 1.16)
    triggerTerrainCenterPulse(time, centerStrength)
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
      (ripple.kind === 'spark' ? 4.45 : ripple.kind === 'snare' ? 4.15 : TERRAIN_PULSE_SPEED) *
      (0.9 + waveTuning * 0.08) *
      ripple.speedScale
    const radius = age * speed
    const speedWidth = 0.94 + ripple.speedScale * 0.06
    const width =
      (ripple.kind === 'spark' ? 0.12 : ripple.kind === 'snare' ? 0.16 : TERRAIN_PULSE_WIDTH) *
      (0.76 + waveTuning * 0.24) *
      speedWidth
    const fadeDistance =
      (ripple.kind === 'spark' ? 2.65 : ripple.kind === 'snare' ? 3.2 : TERRAIN_PULSE_FADE_DISTANCE) *
      (0.82 + waveTuning * 0.18) *
      (0.9 + ripple.speedScale * 0.1)
    const fade = Math.exp(-radius / fadeDistance) * (1 - smoothstep(maxAge * 0.76, maxAge, age))
    ripples.push({ ...ripple, radius, width, fade })
  }
  return ripples
}

function activeTerrainCenterPulse(time: number, reducedMotion: boolean, waveTuning: number) {
  const maxAge = (reducedMotion ? 3.25 : 3.65) * (0.86 + waveTuning * 0.14)
  const age = time - terrainCenterPulseStartedAt
  if (age < 0 || age > maxAge || terrainCenterPulseStrength <= 0.08) {
    return inactiveTerrainCenterPulse()
  }

  const speed =
    TERRAIN_CENTER_PULSE_SPEED *
    (0.9 + waveTuning * 0.08) *
    terrainCenterPulseSpeedScale
  const radius = age * speed
  const speedWidth = 0.94 + terrainCenterPulseSpeedScale * 0.06
  const width = TERRAIN_PULSE_WIDTH * (0.76 + waveTuning * 0.24) * speedWidth
  const fadeDistance =
    TERRAIN_PULSE_FADE_DISTANCE *
    (0.82 + waveTuning * 0.18) *
    (0.9 + terrainCenterPulseSpeedScale * 0.1)
  const fade = Math.exp(-radius / fadeDistance) * (1 - smoothstep(maxAge * 0.76, maxAge, age))

  return { radius, width, fade, strength: terrainCenterPulseStrength }
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
