<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import DrawerWindow from './windows/DrawerWindow.vue'
import PetMenuWindow from './windows/PetMenuWindow.vue'
import PetWindow from './windows/PetWindow.vue'

const windowLabel = ref('drawer')

onMounted(() => {
  try {
    windowLabel.value = getCurrentWindow().label
  } catch {
    windowLabel.value = 'drawer'
  }
})
</script>

<template>
  <PetWindow v-if="windowLabel === 'pet'" />
  <PetMenuWindow v-else-if="windowLabel === 'pet-menu'" />
  <DrawerWindow v-else />
</template>
