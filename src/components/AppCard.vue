<script setup lang="ts">
import { computed } from 'vue'
import type { PetApp } from '../types/app'
import { formatLaunchTime } from '../utils/format'

const props = defineProps<{
  app: PetApp
  tagDisplayMode: 'compact' | 'detailed'
}>()

const emit = defineEmits<{
  launch: [app: PetApp]
  edit: [app: PetApp]
  remove: [app: PetApp]
  'open-dir': [app: PetApp]
}>()

const initials = computed(() => props.app.name.slice(0, 2).toUpperCase())
const isCompact = computed(() => props.tagDisplayMode === 'compact')
const visibleTags = computed(() => props.app.tags)
</script>

<template>
  <article class="app-card" :class="{ compact: isCompact }">
    <button
      v-if="isCompact"
      class="app-compact-main"
      type="button"
      :title="`启动 ${app.name}`"
      :aria-label="`启动 ${app.name}`"
      @click="emit('launch', app)"
    >
      <span class="app-icon" aria-hidden="true">
        <img v-if="app.iconDataUrl" :src="app.iconDataUrl" alt="" />
        <span v-else>{{ initials }}</span>
      </span>
      <span class="app-compact-info">
        <span class="app-compact-name">{{ app.name }}</span>
        <span class="app-tags compact" v-if="app.tags.length > 0">
          <span v-for="tag in visibleTags" :key="tag">{{ tag }}</span>
        </span>
      </span>
    </button>

    <template v-else>
      <button class="app-main" type="button" @click="emit('launch', app)">
        <span class="app-icon" aria-hidden="true">
          <img v-if="app.iconDataUrl" :src="app.iconDataUrl" alt="" />
          <span v-else>{{ initials }}</span>
        </span>
        <span class="app-info">
          <span class="app-title">{{ app.name }}</span>
          <span class="app-meta">{{ app.category }} · {{ app.launchCount }} 次</span>
        </span>
        <span v-if="app.favorite" class="favorite-mark">常用</span>
      </button>
      <div class="app-tags detailed" v-if="app.tags.length > 0">
        <span v-for="tag in visibleTags" :key="tag">{{ tag }}</span>
      </div>
      <p class="app-path" :title="app.path">{{ app.path }}</p>
      <p class="app-last">最近：{{ formatLaunchTime(app.lastLaunchAt) }}</p>
      <div class="app-actions">
        <button type="button" @click="emit('edit', app)">编辑</button>
        <button type="button" @click="emit('open-dir', app)">目录</button>
        <button class="danger-button" type="button" @click="emit('remove', app)">删除</button>
      </div>
    </template>
  </article>
</template>
