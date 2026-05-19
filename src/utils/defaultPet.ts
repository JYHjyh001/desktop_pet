import type { PetAnimationSet } from '../types/app'

export const defaultPetAnimations: Required<PetAnimationSet> = {
  idle: new URL('../assets/pets/default/idle.svg', import.meta.url).href,
  hover: new URL('../assets/pets/default/hover.svg', import.meta.url).href,
  click: new URL('../assets/pets/default/click.svg', import.meta.url).href,
  dragging: new URL('../assets/pets/default/dragging.svg', import.meta.url).href,
}

export const defaultPetPreview = new URL('../assets/pets/default/preview.svg', import.meta.url).href
