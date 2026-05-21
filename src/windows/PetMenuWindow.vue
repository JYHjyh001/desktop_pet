<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'

async function runAction(action: 'chat' | 'drawer' | 'hidePet' | 'quit') {
  await invoke('hide_pet_menu')

  if (action === 'chat') {
    await invoke('show_pet_chat')
    return
  }

  if (action === 'drawer') {
    await invoke('show_drawer')
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
  <main class="pet-menu-window" @mouseleave="hideMenu">
    <button type="button" @click="runAction('chat')">对话</button>
    <button type="button" @click="runAction('drawer')">打开抽屉</button>
    <button type="button" @click="runAction('hidePet')">隐藏宠物</button>
    <button type="button" @click="runAction('quit')">退出程序</button>
  </main>
</template>
