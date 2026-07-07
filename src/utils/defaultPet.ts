import type { PetAnimationKey, PetSkinSummary } from '../types/app'

type BuiltinPetAnimations = Record<PetAnimationKey, string>

export const petAnimationFields: Array<{
  key: PetAnimationKey
  label: string
  required?: boolean
}> = [
  { key: 'idle', label: '待机动画', required: true },
  { key: 'hover', label: '选中动画' },
  { key: 'click', label: '点击动画' },
  { key: 'dragging', label: '拖动动画' },
  { key: 'draggingLeft', label: '向左拖动动画' },
  { key: 'draggingRight', label: '向右拖动动画' },
  { key: 'waving', label: '打招呼动画' },
  { key: 'jumping', label: '跳跃动画' },
  { key: 'waiting', label: '等待动画' },
  { key: 'running', label: '处理中动画' },
  { key: 'review', label: '检查动画' },
  { key: 'failed', label: '失败动画' },
]

const petAnimationFallbackKeys: Record<PetAnimationKey, PetAnimationKey> = {
  idle: 'idle',
  hover: 'hover',
  click: 'click',
  dragging: 'dragging',
  draggingLeft: 'dragging',
  draggingRight: 'dragging',
  waving: 'hover',
  jumping: 'click',
  waiting: 'idle',
  running: 'hover',
  review: 'idle',
  failed: 'click',
}

export const defaultPetAnimations: BuiltinPetAnimations = {
  idle: new URL('../assets/pets/default/idle.svg', import.meta.url).href,
  hover: new URL('../assets/pets/default/hover.svg', import.meta.url).href,
  click: new URL('../assets/pets/default/click.svg', import.meta.url).href,
  dragging: new URL('../assets/pets/default/dragging.svg', import.meta.url).href,
  draggingLeft: new URL('../assets/pets/default/dragging.svg', import.meta.url).href,
  draggingRight: new URL('../assets/pets/default/dragging.svg', import.meta.url).href,
  waving: new URL('../assets/pets/default/hover.svg', import.meta.url).href,
  jumping: new URL('../assets/pets/default/click.svg', import.meta.url).href,
  waiting: new URL('../assets/pets/default/idle.svg', import.meta.url).href,
  running: new URL('../assets/pets/default/hover.svg', import.meta.url).href,
  review: new URL('../assets/pets/default/idle.svg', import.meta.url).href,
  failed: new URL('../assets/pets/default/click.svg', import.meta.url).href,
}

export const defaultPetPreview = new URL('../assets/pets/default/preview.svg', import.meta.url).href

const builtinPetAssets: Record<string, { preview: string; animations: BuiltinPetAnimations }> = {
  default: {
    preview: defaultPetPreview,
    animations: defaultPetAnimations,
  },
}

export function getBuiltinPetAnimations(skinId?: string | null): BuiltinPetAnimations {
  return builtinPetAssets[skinId || 'default']?.animations ?? defaultPetAnimations
}

export function getBuiltinPetPreview(skinId?: string | null): string {
  return builtinPetAssets[skinId || 'default']?.preview ?? defaultPetPreview
}

export function getPetSkinPreview(skin: PetSkinSummary | null | undefined): string {
  if (skin?.preview) {
    return skin.preview
  }

  if (skin?.builtin) {
    return getBuiltinPetPreview(skin.id)
  }

  return defaultPetPreview
}

export function getPetSkinAnimation(
  skin: PetSkinSummary | null | undefined,
  key: PetAnimationKey,
): string {
  const configured = skin?.animations?.[key]
  if (configured) {
    return configured
  }

  if (skin?.builtin) {
    return getBuiltinPetAnimations(skin.id)[key]
  }

  return ''
}

export function resolvePetSkinAnimations(skin: PetSkinSummary | null | undefined): BuiltinPetAnimations {
  const fallback = skin?.builtin ? getBuiltinPetAnimations(skin.id) : defaultPetAnimations
  const animations = skin?.animations ?? {}

  return petAnimationFields.reduce((resolved, field) => {
    const key = field.key
    const fallbackKey = petAnimationFallbackKeys[key]
    resolved[key] =
      animations[key] ||
      animations[fallbackKey] ||
      animations.idle ||
      fallback[key] ||
      fallback[fallbackKey] ||
      fallback.idle
    return resolved
  }, {} as BuiltinPetAnimations)
}
