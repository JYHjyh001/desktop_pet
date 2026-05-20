import type { PetAnimationSet, PetSkinSummary } from '../types/app'

type PetAnimationKey = keyof PetAnimationSet
type BuiltinPetAnimations = Record<PetAnimationKey, string>

export const defaultPetAnimations: BuiltinPetAnimations = {
  idle: new URL('../assets/pets/default/idle.svg', import.meta.url).href,
  hover: new URL('../assets/pets/default/hover.svg', import.meta.url).href,
  click: new URL('../assets/pets/default/click.svg', import.meta.url).href,
  dragging: new URL('../assets/pets/default/dragging.svg', import.meta.url).href,
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

  return {
    idle: animations.idle || fallback.idle,
    hover: animations.hover || animations.idle || fallback.hover,
    click: animations.click || animations.idle || fallback.click,
    dragging: animations.dragging || animations.idle || fallback.dragging,
  }
}
