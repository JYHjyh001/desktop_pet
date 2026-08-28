<script setup lang="ts">
import { defineAsyncComponent } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'

const DrawerWindow = defineAsyncComponent(() => import('./windows/DrawerWindow.vue'))
const MusicWindow = defineAsyncComponent(() => import('./windows/MusicWindow.vue'))
const PetBubbleWindow = defineAsyncComponent(() => import('./windows/PetBubbleWindow.vue'))
const PetChatWindow = defineAsyncComponent(() => import('./windows/PetChatWindow.vue'))
const PetMenuWindow = defineAsyncComponent(() => import('./windows/PetMenuWindow.vue'))
const PetWindow = defineAsyncComponent(() => import('./windows/PetWindow.vue'))
const StoryWindow = defineAsyncComponent(() => import('./windows/StoryWindow.vue'))
const TranslatorWindow = defineAsyncComponent(() => import('./windows/TranslatorWindow.vue'))

const windowLabel = (() => {
  try {
    return getCurrentWindow().label
  } catch {
    return 'drawer'
  }
})()
</script>

<template>
  <PetWindow v-if="windowLabel === 'pet'" />
  <PetBubbleWindow v-else-if="windowLabel === 'pet-bubble'" />
  <PetMenuWindow v-else-if="windowLabel === 'pet-menu'" />
  <PetChatWindow v-else-if="windowLabel === 'pet-chat'" />
  <StoryWindow v-else-if="windowLabel === 'story'" />
  <TranslatorWindow v-else-if="windowLabel === 'translator'" />
  <MusicWindow v-else-if="windowLabel === 'music'" />
  <DrawerWindow v-else />
</template>
