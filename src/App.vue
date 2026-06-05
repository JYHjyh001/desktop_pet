<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import DrawerWindow from './windows/DrawerWindow.vue'
import PetChatWindow from './windows/PetChatWindow.vue'
import PetMenuWindow from './windows/PetMenuWindow.vue'
import PetWindow from './windows/PetWindow.vue'
import StoryWindow from './windows/StoryWindow.vue'

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
  <PetChatWindow v-else-if="windowLabel === 'pet-chat'" />
  <StoryWindow v-else-if="windowLabel === 'story'" />
  <DrawerWindow v-else />
</template>
