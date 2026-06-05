<script setup lang="ts">
import { computed } from 'vue'
import type { PetApp } from '../types/app'
import { formatLaunchTime } from '../utils/format'

const folderDefaultIcon = new URL('../assets/icons/default-folder.svg', import.meta.url).href
const websiteDefaultIcon = new URL('../assets/icons/default-website.svg', import.meta.url).href
const fileDefaultIcon = new URL('../assets/icons/default-file.svg', import.meta.url).href

const props = defineProps<{
  app: PetApp
  tagDisplayMode: 'compact' | 'detailed'
}>()

const emit = defineEmits<{
  launch: [app: PetApp]
  edit: [app: PetApp]
  remove: [app: PetApp]
  'open-dir': [app: PetApp]
  'toggle-admin': [app: PetApp, runAsAdmin: boolean]
}>()

const initials = computed(() => props.app.name.slice(0, 2).toUpperCase())
const isCompact = computed(() => props.tagDisplayMode === 'compact')
const visibleTags = computed(() => props.app.tags)
const compactVisibleTags = computed(() => visibleTags.value.slice(0, 1))
const compactHiddenTagCount = computed(() =>
  Math.max(visibleTags.value.length - compactVisibleTags.value.length, 0),
)
const itemKindLabel = computed(() => {
  if (props.app.itemKind === 'folder') {
    return '文件夹'
  }

  if (props.app.itemKind === 'website') {
    return '网站'
  }

  if (props.app.itemKind === 'file') {
    return '文件'
  }

  return '软件'
})
const targetLabel = computed(() => (props.app.itemKind === 'website' ? '网址' : '路径'))
const defaultIconUrl = computed(() => {
  if (props.app.itemKind === 'folder') {
    return folderDefaultIcon
  }

  if (props.app.itemKind === 'website') {
    return websiteDefaultIcon
  }

  if (props.app.itemKind === 'file') {
    return fileDefaultIcon
  }

  return null
})
const displayedIconUrl = computed(() =>
  props.app.itemKind === 'app' ? props.app.iconDataUrl : defaultIconUrl.value,
)
const launchTitle = computed(() =>
  props.app.runAsAdmin ? `以管理员身份启动 ${props.app.name}` : `打开 ${props.app.name}`,
)
const compactTagsTitle = computed(() => {
  const parts = [`类型：${itemKindLabel.value}`]

  if (props.app.runAsAdmin) {
    parts.push('管理员启动')
  }

  if (visibleTags.value.length > 0) {
    parts.push(`标签：${visibleTags.value.join('，')}`)
  }

  return parts.join('；')
})

function onAdminToggle(event: Event) {
  const input = event.target as HTMLInputElement
  const runAsAdmin = input.checked
  input.checked = props.app.runAsAdmin
  emit('toggle-admin', props.app, runAsAdmin)
}
</script>

<template>
  <article class="app-card" :class="[{ compact: isCompact }, `kind-${app.itemKind}`]">
    <template v-if="isCompact">
      <button
        class="app-compact-main"
        type="button"
        :title="launchTitle"
        :aria-label="launchTitle"
        @click="emit('launch', app)"
      >
        <span class="app-icon" aria-hidden="true">
          <img v-if="displayedIconUrl" :src="displayedIconUrl" alt="" />
          <span v-else>{{ initials }}</span>
        </span>
        <span class="app-compact-info">
          <span class="app-compact-name">{{ app.name }}</span>
          <span class="app-tags compact" :title="compactTagsTitle">
            <span class="kind-tag">{{ itemKindLabel }}</span>
            <span v-for="tag in compactVisibleTags" :key="tag">{{ tag }}</span>
            <span v-if="compactHiddenTagCount > 0" class="tag-more">
              +{{ compactHiddenTagCount }}
            </span>
          </span>
        </span>
      </button>

      <label v-if="app.itemKind === 'app'" class="app-admin-toggle" @click.stop>
        <input :checked="app.runAsAdmin" type="checkbox" @change="onAdminToggle" />
        管理员启动
      </label>
    </template>

    <template v-else>
      <button class="app-main" type="button" :title="launchTitle" @click="emit('launch', app)">
        <span class="app-icon" aria-hidden="true">
          <img v-if="displayedIconUrl" :src="displayedIconUrl" alt="" />
          <span v-else>{{ initials }}</span>
        </span>
        <span class="app-info">
          <span class="app-title">{{ app.name }}</span>
          <span class="app-meta">
            {{ itemKindLabel }} · {{ app.category }} · {{ app.launchCount }} 次
            <template v-if="app.runAsAdmin"> · 管理员启动</template>
          </span>
        </span>
        <span v-if="app.favorite" class="favorite-mark">常用</span>
      </button>
      <div class="app-tags detailed" v-if="app.tags.length > 0">
        <span v-for="tag in visibleTags" :key="tag">{{ tag }}</span>
      </div>
      <p class="app-path" :title="app.path">{{ targetLabel }}：{{ app.path }}</p>
      <p class="app-last">最近：{{ formatLaunchTime(app.lastLaunchAt) }}</p>
      <div class="app-actions">
        <label v-if="app.itemKind === 'app'" class="app-admin-toggle" @click.stop>
          <input :checked="app.runAsAdmin" type="checkbox" @change="onAdminToggle" />
          管理员启动
        </label>
        <button type="button" @click="emit('edit', app)">编辑</button>
        <button v-if="app.itemKind !== 'website'" type="button" @click="emit('open-dir', app)">
          {{ app.itemKind === 'folder' ? '打开' : '目录' }}
        </button>
        <button class="danger-button" type="button" @click="emit('remove', app)">删除</button>
      </div>
    </template>
  </article>
</template>
