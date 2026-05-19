<script setup lang="ts">
defineProps<{
  state: 'idle' | 'hover' | 'dragging' | 'click'
  imageUrl?: string
}>()
</script>

<template>
  <div class="pet-figure" :class="`pet-${state}`" aria-label="PetDrawer 桌面宠物">
    <img v-if="imageUrl" class="custom-pet-image" :src="imageUrl" alt="" />
    <template v-else>
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
    </template>
  </div>
</template>
