<script setup lang="ts">
import { computed } from 'vue'
import type { PetAnimationKey } from '../types/app'

const props = defineProps<{
  state: PetAnimationKey
  imageUrl?: string
}>()

const isVideo = computed(() => {
  const source = props.imageUrl ?? ''
  return /^data:video\//i.test(source) || /\.(webm|mp4)(?:[?#].*)?$/i.test(source)
})

const mediaKey = computed(() => {
  if (!props.imageUrl) {
    return 'default'
  }

  return `${isVideo.value ? 'video' : 'image'}:${props.imageUrl}`
})
</script>

<template>
  <div class="pet-figure" :class="`pet-${state}`" aria-label="PetDrawer 桌面宠物">
    <Transition name="pet-media">
      <video
        v-if="imageUrl && isVideo"
        :key="mediaKey"
        class="custom-pet-image"
        :src="imageUrl"
        autoplay
        loop
        muted
        playsinline
        preload="auto"
        aria-hidden="true"
      />
      <img v-else-if="imageUrl" :key="mediaKey" class="custom-pet-image" :src="imageUrl" alt="" />
      <div v-else :key="mediaKey" class="default-pet-layer">
        <div class="pet-ear pet-ear-left" />
        <div class="pet-ear pet-ear-right" />
        <div class="pet-head">
          <div class="pet-eye pet-eye-left" />
          <div class="pet-eye pet-eye-right" />
          <div class="pet-blush pet-blush-left" />
          <div class="pet-blush pet-blush-right" />
          <div class="pet-mouth" />
        </div>
        <div class="pet-body">
          <div class="pet-badge" />
        </div>
        <div class="pet-shadow" />
      </div>
    </Transition>
  </div>
</template>
