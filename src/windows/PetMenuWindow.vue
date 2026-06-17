<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { DrawerTheme, PetDrawerConfig } from '../types/app'

const drawerTheme = ref<DrawerTheme>('light')
const themeClass = computed(() => `theme-${drawerTheme.value}`)
let unlistenThemeChanged: (() => void) | null = null

onMounted(async () => {
  try {
    const config = await invoke<PetDrawerConfig>('get_config')
    drawerTheme.value = config.drawer.theme === 'animal-island' ? 'animal-island' : 'light'
  } catch {
    drawerTheme.value = 'light'
  }

  unlistenThemeChanged = await listen<string>('ui-theme-changed', (event) => {
    drawerTheme.value = event.payload === 'animal-island' ? 'animal-island' : 'light'
  })
})

onBeforeUnmount(() => {
  unlistenThemeChanged?.()
})

async function runAction(action: 'chat' | 'story' | 'drawer' | 'music' | 'hidePet' | 'quit') {
  await invoke('hide_pet_menu')

  if (action === 'chat') {
    await invoke('show_pet_chat')
    return
  }

  if (action === 'story') {
    await invoke('show_story')
    return
  }

  if (action === 'drawer') {
    await invoke('show_drawer')
    return
  }

  if (action === 'music') {
    await invoke('show_music_player')
    return
  }

  if (action === 'hidePet') {
    await invoke('hide_pet')
    return
  }

  await invoke('quit_app')
}

async function hideMenu() {
  await invoke('hide_pet_menu')
}
</script>

<template>
  <main class="pet-menu-window" :class="themeClass" @mouseleave="hideMenu">
    <button type="button" @click="runAction('chat')">对话</button>
    <button type="button" @click="runAction('story')">故事模式</button>
    <button type="button" @click="runAction('drawer')">打开抽屉</button>
    <button type="button" @click="runAction('music')">音乐播放</button>
    <button type="button" @click="runAction('hidePet')">隐藏宠物</button>
    <button type="button" @click="runAction('quit')">退出程序</button>
  </main>
</template>
